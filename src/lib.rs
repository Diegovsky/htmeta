#![feature(let_chains)]
#![doc = include_str!("../README.md")]

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
    io::Write
};

pub use kdl;

use kdl::{KdlDocument, KdlNode, KdlValue};
use regex::Captures;

pub struct PluginContext<'a, 'b: 'a> {
    pub indent: &'a str,
    pub writer: &'a mut Writer<'b>,
    pub emitter: &'a HtmlEmitter<'a>
}

pub trait IPlugin {
    fn dyn_clone(&self) -> Box<dyn IPlugin>;
    fn emit_node(&self, node: &KdlNode, context: PluginContext) -> EmitResult<bool>;
}

pub type Writer<'a> = &'a mut dyn Write;
type Text<'b> = Cow<'b, str>;
pub type EmitResult<T = ()> = Result<T, Error>;

struct Plugin(Box<dyn IPlugin>);

impl Plugin {
    pub fn new<P: IPlugin + 'static>(plugin: P) -> Self {
        Self(Box::new(plugin))
    }
}

impl Clone for Plugin {
    fn clone(&self) -> Self {
        Self(self.0.dyn_clone())
    }
}

pub type Indent = usize;

mod error;

pub use error::Error;

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

    /// Registers a plugin for all instances of this builder.
    pub fn add_plugin<P: IPlugin + 'static>(&mut self, plugin: P) -> &mut Self {
        self.plugins.push(Plugin::new(plugin));
        self
    }

    /// Creates a new [`HtmlEmitter`]. You should re-use this builder to create emitters
    /// efficiently.
    pub fn build<'a>(&self) -> HtmlEmitter<'a> {
        HtmlEmitter {
            current_level: 0,
            indent: self.indent,
            plugins: self.plugins.clone(),
            vars: Default::default(),
        }
    }

    /* pub fn sex(&self, text: &str) -> HtmlEmitter<'> {
        
    } */
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
    pub indent: Indent,
    pub current_level: Indent,
    pub vars: HashMap<&'a str, Text<'a>>,
    plugins: Vec<Plugin>,
}

impl<'a> HtmlEmitter<'a> {
    /// A convenience method that just calls [`HtmlEmitterBuilder::new`].
    ///
    /// Check out that type's documentation for uses!
    pub fn builder() -> HtmlEmitterBuilder {
        HtmlEmitterBuilder::new()
    }

    /// Returns an [`HtmlEmitter`] with a copy of `self`'s variables and one indentation level
    /// deeper. This emitter should be uses to translate a child of `self`.
    pub fn subemitter(&self) -> Self {
        Self {
            current_level: self.current_level + 1,
            // node,
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
        re!(VAR, r"\$(\w+)");
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
        &self,
        node: &KdlNode,
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
                self.subemitter().emit(doc, writer)?;
                write!(writer, "{}", indent)?;
            }
            write!(writer, "</{}>", name)?;
            self.write_line(writer)?;
        }
        Ok(())
    }

    fn call_plugin<'b: 'a>(&'b self, node: &'a KdlNode, indent: &'b str, mut writer: Writer<'b>) -> EmitResult<bool> {
        for plug in &self.plugins {
            let ctx = PluginContext {
                indent, 
                emitter: self,
                writer: &mut writer,
            };
            if plug.0.emit_node(node, ctx)? {
                return Ok(true)
            }
        }
        Ok(false)
    }

    pub fn emit_text_node(&self, indent: &str, content: &KdlValue, writer: Writer) -> EmitResult {
        write!(
            writer,
            "{}{}",
            indent,
            html_escape::encode_text(&self.expand_value(content))
        )?;
        self.write_line(writer)?;
        Ok(())
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
    pub fn emit<'b: 'a>(&mut self, document: &'b KdlDocument, writer: Writer) -> EmitResult {
        let indent = self.indent();

        for node in document.nodes() {
            let name = node.name().value();

            // variable node
            if name.starts_with("$")
                && let Some(val) = node.get(0)
            {
                self.vars.insert(&name[1..], self.expand_value(val.value()));
                continue;
            }

            // text node
            if name == "text"
                && let Some(content) = node.get(0)
            {
                self.emit_text_node(&indent, content.value(), writer)?;
                continue;
            }

            // Plugin shenanigans
            if self.call_plugin(node, &indent, writer)? {
                continue
            }

            // Compound node, AKA, normal HTML tag.
            self.emit_tag(node, name, &indent, writer)?
        }
        // Allows this instance to be reused
        self.vars.clear();
        Ok(())
    }
}


#[doc(hidden)]
pub fn emit_as_str(builder: &HtmlEmitterBuilder, input: &str) -> String {
    let doc: kdl::KdlDocument = input.parse().expect("Failed to parse as kdl doc");
    let mut buf = Vec::<u8>::new();
    let mut emitter = builder.build();
    emitter.emit(&doc, &mut buf).expect("Failed to emit HTML");
    String::from_utf8(buf).expect("Invalid utf8 found")
}
#[cfg(test)]
pub mod tests {
    use super::*;
    use htmeta_auto_test::*;

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

    struct ShouterPlugin;

    impl IPlugin for ShouterPlugin {
        fn dyn_clone(&self) -> Box<dyn IPlugin> {
            Box::new(ShouterPlugin)
        }
        fn emit_node(&self, node: &KdlNode, context: PluginContext) -> EmitResult<bool> {
            let name = node.name().value();
            context.emitter.emit_tag(node, &name.to_uppercase(), context.indent, context.writer)?;
            Ok(true)
        }
    }

    fn with_plugin() -> HtmlEmitterBuilder {
        let mut builder = HtmlEmitter::builder();
        builder.add_plugin(ShouterPlugin);
        builder
    }

    auto_html_test!(shouter_basic, with_plugin());
}
