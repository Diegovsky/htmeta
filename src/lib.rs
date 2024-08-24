#![feature(let_chains)]
#![doc(include_str!("../README.md"))]

macro_rules! re {
    ($name:ident, $e:expr) => {
        use regex::Regex;
        use std::sync::LazyLock;
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($e).unwrap());
    };
}

use std::{
    borrow::Cow,
    collections::HashMap,
    io::{self, Write}, rc::Rc,
};

use kdl::{KdlDocument, KdlNode, KdlValue};
use regex::Captures;

pub type Writer<'a> = &'a mut dyn Write;
type Text<'b> = Cow<'b, str>;
pub type EmitResult<T = ()> = io::Result<T>;
pub type Plugin = Rc<dyn for<'a, 'b> Fn(&'a HtmlEmitter, &'a Writer<'b>) -> EmitResult<bool>>;
pub type Indent = usize;

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr"
];

/// A builder for [`HtmlEmitter`]s.
#[derive(Clone, Default)]
pub struct HtmlEmitterBuilder {
    indent: Indent,
    plugins: Vec<Plugin>,
}

impl HtmlEmitterBuilder {
    /// Returns a new [`Self`] instance with a default indentation value of 4.
    pub fn new() -> Self {
        Self { indent: 4, ..Self::default() }
    }

    /// Sets the indentation amount. Implies pretty formatting.
    pub fn indent(&mut self, indent: Indent) -> &mut Self {
        self.indent = indent;
        self
    }

    /// Disables indentation and newlines. 
    pub fn minify(&mut self) -> &mut Self {
        self.indent = 0;
        self
    }

    /// Registers plugins for all instances of this builder.
    pub fn add_plugins<P>(&mut self, plugins: P) -> &mut Self where P: IntoIterator<Item = Plugin> {
        self.plugins.extend(plugins);
        self
    }

    /// Creates a new [`HtmlEmitter`]. You should re-use this builder to create emitters
    /// efficiently.
    pub fn build<'a>(&self, node: &'a KdlDocument) -> HtmlEmitter<'a> {
        HtmlEmitter {
            node,
            current_level: 0,
            indent: self.indent,
            plugins: self.plugins.clone(),
            vars: Default::default(),
        }
    }
}

/// An `HTML` emitter for `htmeta`.
///
/// ```rust
/// use htmeta::HtmlEmitter;
/// use kdl::KdlDocument;
/// let doc KdlDocument = r#"html { body { h1 { text "Title" }}}"#.parse().unwrap();
///
/// // Creates an emitter with an indentation level of 4.
/// let emitter = HtmlEmitter::builder().indent(4).build(&doc);
///
/// // Emits html to the terminal.
/// emitter.emit(std::io::stdout()).unwrap();
/// ```
#[derive(Clone)]
pub struct HtmlEmitter<'a> {
    pub node: &'a KdlDocument,
    pub indent: Indent,
    pub current_level: Indent,
    pub vars: HashMap<&'a str, Text<'a>>,
    plugins: Vec<Plugin>,
}

impl<'a> HtmlEmitter<'a> {
    #[deprecated = "Use the builder interface [`Self::builder()`]"]
    /// Creates a new [`HtmlEmitter`] with an indentation level of `indent`.
    ///
    /// This is deprecated. Use [`Self::builder`] instead.
    pub fn new(node: &'a KdlDocument, indent: usize) -> Self {
        Self::builder().indent(indent).build(node)
    }

    /// A convenience method that just calls [`HtmlEmitterBuilder::new`].
    ///
    /// Check out that type's documentation for uses!
    pub fn builder() -> HtmlEmitterBuilder {
        HtmlEmitterBuilder::new()
    }

    /// Returns an [`HtmlEmitter`] with a copy of `self`'s variables and one indentation level
    /// deeper. This emitter should be uses to translate a child of `self`.
    pub fn subemitter(&self, node: &'a KdlDocument) -> Self {
        Self {
            current_level: self.current_level + 1,
            node,
            ..self.clone()
        }
    }

    /// Returns `true` if in pretty mode, `false` otherwise.
    pub fn is_pretty(&self) -> bool {
        self.indent > 0
    }

    /// Convenience function that writes a newline if in pretty mode.
    pub fn write_line(&self, writer: Writer) -> EmitResult {
        if self.is_pretty() {
            writeln!(writer)?;
        }
        Ok(())
    }

    /// Convenience function that returns a new [`String`] containing the current indentation
    /// level's worth of spaces.
    ///
    /// # Example
    /// ```rust no_test
    /// use htmeta::HtmlEmitter;
    /// let emitter = HtmlEmitter::builder().indent(4).build();
    /// assert_eq!(emitter.indent(), "");
    /// ```
    pub fn indent(&self) -> String {
        " ".repeat(self.current_level * self.indent)
    }

    /// Replaces all occurences of variables inside `text` and returns a new string.
    pub fn expand_string<'b>(&self, text: &'b str) -> Text<'b> {
        re!(VAR, r"(\$\w+)");
        VAR.replace(text, |captures: &Captures| {
            self.vars
                .get(&captures[1])
                .map(ToString::to_string)
                .unwrap_or_default()
        })
    }

    /// Converts the `value`'s [`String`] representation and replaces any variables found within.
    /// This is a convenient wrapper around [`Self::expand_string`].
    pub fn expand_value<'b>(&self, value: &'b KdlValue) -> Text<'b> {
        match value {
            KdlValue::RawString(content) | KdlValue::String(content) => self.expand_string(content),
            _ => todo!(),
        }
    }

    /// Emits a compound `HTML` tag named `name`, with `indent` as indentation, using `node` for
    /// properties and children.
    pub fn emit_tag(
        &mut self,
        node: &'a KdlNode,
        name: &str,
        indent: &str,
        writer: Writer
    ) -> EmitResult {
        let is_void = VOID_TAGS.contains(&name);

        // opening tag
        write!(writer, "{}<{}", indent, name)?;
        // args
        let args = node
            .entries()
            .iter()
            .map(|arg| self.expand_string(&arg.to_string()).into_owned())
            .collect::<Vec<_>>()
            .join("");

        write!(writer, "{}", args)?;

        if is_void {
            write!(writer, "/>")?;
            self.write_line(writer)?;
        } else {
            write!(writer, ">")?;
            // Children
            if let Some(doc) = node.children() {
                self.write_line(writer)?;
                self.subemitter(doc).emit(writer)?;
                write!(writer, "{}", indent)?;
            }
            write!(writer, "</{}>", name)?;
            self.write_line(writer)?;
        }
        Ok(())
    }

    fn call_plugin(&self, mut writer: Writer) -> EmitResult<bool> {
        for plug in &self.plugins {
            if (*plug)(self, &mut writer)? {
                return Ok(true)
            }
        }
        Ok(false)
    }

    /// Emits the corresponding `HTML` into the `writer`. The emitter can be re-used after this.
    ///
    /// # Examples:
    ///
    /// ```rust
    /// use htmeta::HtmlEmitter;
    /// use kdl::KdlDocument;
    /// let doc: KdlDocument = r#"html { body { h1 { text "Title" }}}"#.parse().unwrap();
    /// // Creates an emitter with an indentation level of 4.
    /// let emitter = HtmlEmitter::new(&doc, 4);
    /// // You should wrap this with a `BufWriter` for actual use.
    /// let file = std::fs::File::create("index.html").unwrap();
    /// emitter.emit(&mut file).unwrap();
    /// ```
    pub fn emit(&mut self, writer: Writer<'a>) -> EmitResult {
        let indent = self.indent();

        for node in self.node.nodes() {
            let name = node.name().value();

            // Plugin shenanigans
            if self.call_plugin(writer)? {
                continue
            }

            // variable node
            if name.starts_with("$")
                && let Some(val) = node.get(0)
            {
                self.vars.insert(name, self.expand_value(val.value()));
                continue;
            }

            // text node
            if name == "text"
                && let Some(content) = node.get(0)
            {
                write!(
                    writer,
                    "{}{}",
                    indent,
                    html_escape::encode_text(&self.expand_value(content.value()))
                )?;
                self.write_line(writer)?;
                continue;
            }

            // Compound node, AKA, normal HTML tag.
            self.emit_tag(node, name, &indent, writer)?
        }
        // Allows this instance to be reused
        self.vars.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;

    fn emit_as_str(builder: &HtmlEmitterBuilder, input: &str) -> String {
        let doc: KdlDocument = input.parse().expect("Failed to parse as kdl doc");
        let mut buf = Vec::<u8>::new();
        let mut emitter = builder.build(&doc);
        emitter.emit(&mut buf).expect("Failed to emit HTML");
        String::from_utf8(buf).expect("Invalid utf8 found")
    }

    macro_rules! include_fixture {
        ($expr:expr) => {
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/", $expr))
        };
    }

    macro_rules! auto_html_test {
        ($name:ident) => {
            auto_html_test!($name, HtmlEmitter::builder());
        };
        ($name:ident, $builder: expr) => {
            #[test]
            fn $name() {
                let input = include_fixture!(concat!(stringify!($name), ".kdl"));

                let builder = $builder;
                let result = emit_as_str(&builder, input);
                assert_eq!(result, include_fixture!(concat!(stringify!($name), ".html")));
            }
        };
    }


    auto_html_test!(basic_test);
    auto_html_test!(basic_var);
    auto_html_test!(var_scopes);

    fn minified() -> HtmlEmitterBuilder {
        let mut builder = HtmlEmitter::builder();
        builder.minify();
        builder
    }

    auto_html_test!(minified_basic, minified());
    auto_html_test!(minified_var_scopes, minified());

}
