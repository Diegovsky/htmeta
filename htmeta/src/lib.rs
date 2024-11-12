#![feature(let_chains)]
#![doc = include_str!("../../README.md")]

macro_rules! re {
    ($name:ident, $e:expr) => {
        use regex::Regex;
        use std::sync::LazyLock;
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($e).unwrap());
    };
}

use std::{borrow::Cow, collections::HashMap, io::Write, rc::Rc};

use dyn_clone::DynClone;
pub use kdl;

use kdl::{KdlDocument, KdlNode, KdlValue};
use regex::Captures;

/// Convenient alias for a [`std::io::Write`] mutable reference.
pub type Writer<'a> = &'a mut dyn Write;

/// Convenient alias for this crate's return types.
pub type EmitResult<T = ()> = Result<T, Error>;

/// The type used to represent indentation length.
///
/// Could change in the future to be more efficient, so please,
/// use this instead of the type it is aliasing!
pub type Indent = usize;

/// Information that plugins can use to change what is being emitted.
///
/// Check out [`HtmlEmitter`] for more information!
pub struct PluginContext<'a, 'b: 'a> {
    /// Pre-computed indentation from the current level.
    pub indent: &'a str,
    /// The [`Writer`] handle we're currently emitting into.
    pub writer: &'a mut Writer<'b>,
    /// A handle to the current node's emitter.
    pub emitter: &'a HtmlEmitter<'a>,
}

/// Information that plugins can use to change what is being emitted.
///
/// Check out [`HtmlEmitter`] for more information!
pub struct PluginMutContext<'a, 'b: 'a> {
    /// Pre-computed indentation from the current level.
    pub indent: &'a str,
    /// The [`Writer`] handle we're currently emitting into.
    pub writer: &'a mut Writer<'b>,
    /// A mutable handle to the current node's emitter.
    pub emitter: &'a mut HtmlEmitter<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum EmitStatus {
    Skip,
    Emmited,
    NeedsMutation,
}

/// A trait that allows you to hook into `htmeta`'s emitter and extend it!
pub trait IPlugin: DynClone {
    fn emit_node(&self, node: &KdlNode, context: PluginContext) -> EmitResult<EmitStatus>;
    fn emit_node_mut(&mut self, node: &KdlNode, context: PluginContext) -> EmitResult<()> {
        let _ = (node, context);
        unimplemented!("")
    }
}

type Text<'b> = Cow<'b, str>;

#[derive(Clone)]
struct Plugin(Rc<dyn IPlugin>);

impl Plugin {
    pub fn new<P: IPlugin + 'static>(plugin: P) -> Self {
        Self(Rc::new(plugin))
    }

    pub fn make_mut(&mut self) -> &mut dyn IPlugin {
        dyn_clone::rc_make_mut(&mut self.0)
    }
}

mod error;

pub use error::Error;

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr", "!DOCTYPE", // not a tag at all, but works a lot like one.
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
        Self {
            indent: 4,
            ..Self::default()
        }
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
}

type VarMap<'content> = HashMap<Box<str>, Text<'content>>;
#[derive(Clone, Debug, Default)]
pub struct Vars<'content> {
    vars: Rc<VarMap<'content>>,
}

impl<'content> Vars<'content> {
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
            KdlValue::String(content) => self.expand_string(content),
            _ => todo!(),
        }
    }

    fn make_mut(&mut self) -> &mut VarMap<'content> {
        Rc::make_mut(&mut self.vars)
    }

    pub fn insert(&mut self, key: &str, value: Text<'content>) {
        self.make_mut().insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&Text<'content>> {
        self.vars.get(key)
    }

    pub fn clear(&mut self) {
        self.make_mut().clear();
    }
}

impl<'a, S> std::iter::Extend<(S, Text<'a>)> for Vars<'a>
where
    S: Into<Box<str>>,
{
    fn extend<T: IntoIterator<Item = (S, Text<'a>)>>(&mut self, iter: T) {
        self.make_mut()
            .extend(iter.into_iter().map(|(k, v)| (k.into(), v)))
    }
}

/// The `HTML` emitter for `htmeta`.
///
/// ```rust
/// use htmeta::HtmlEmitter;
/// use kdl::KdlDocument;
/// let doc: KdlDocument = r#"
/// html {
///     body {
///         h1 {
///             text "Title"
///         }
///     }
/// }"#.parse().unwrap();
///
/// // Creates an emitter with an indentation level of 4.
/// let mut emitter = HtmlEmitter::builder().indent(4).build();
///
/// // Emits html to the terminal.
/// emitter.emit(&doc, &mut std::io::stdout()).unwrap();
/// ```
#[derive(Clone)]
pub struct HtmlEmitter<'a> {
    pub indent: Indent,
    pub current_level: Indent,
    pub vars: Vars<'a>,
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
    /// deeper. This emitter should be used to translate a child of `self`.
    pub fn subemitter<'b: 'a>(&'b self) -> HtmlEmitter<'b> {
        HtmlEmitter {
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
    /// ```rust
    /// use htmeta::HtmlEmitter;
    /// let emitter = HtmlEmitter::builder().indent(4).build();
    /// assert_eq!(emitter.indent(), "");
    /// ```
    pub fn indent(&self) -> String {
        " ".repeat(self.current_level * self.indent)
    }

    /// Emits a compound `HTML` tag named `name`, with `indent` as indentation, using `node` for
    /// properties and children.
    ///
    /// Despite the unassuming name and description, this emits like 90% of the nodes.
    ///
    /// # Example
    /// ```rust
    /// use htmeta::HtmlEmitter;
    /// use htmeta::kdl::KdlNode;
    ///
    /// let emitter = HtmlEmitter::builder().minify().build();
    /// // Creates a simple paragraph node
    /// let node = r#"p id="paragraph" { text "Hello, world!" }"#.parse::<KdlNode>().unwrap();
    /// let mut result = Vec::<u8>::new();
    /// emitter.emit_tag(&node, node.name().value(), "", &mut result).unwrap();
    /// assert_eq!(result, br#"<p id="paragraph">Hello, world!</p>"#);
    /// ```
    pub fn emit_tag<'b: 'a>(
        &'a self,
        node: &'a KdlNode,
        name: &str,
        indent: &str,
        writer: Writer<'b>,
    ) -> EmitResult {
        let is_void = VOID_TAGS.contains(&name);

        // opening tag
        write!(writer, "{}<{}", indent, name)?;
        // args
        let (contents, args) = node.entries().iter().partition::<Vec<_>, _>(|arg| {
            matches!(
                arg.name().map(|ident| ident.value()),
                Some("content" | "text")
            )
        });

        let args = args
            .iter()
            .map(|arg| self.vars.expand_string(&arg.to_string()).into_owned())
            .collect::<Vec<_>>()
            .join("");

        write!(writer, "{}", args)?;

        if is_void {
            write!(writer, ">")?;
            self.write_line(writer)?;
        } else {
            write!(writer, ">")?;
            // Inline `text`/`content` param
            if let Some(inline) = contents.last() {
                write!(writer, "{}", self.vars.expand_value(inline.value()))?;
            }
            // Children
            if let Some(doc) = node.children() {
                self.write_line(writer)?;
                let mut value = self.subemitter();
                value.emit(doc, writer)?;
                write!(writer, "{}", indent)?;
            }
            write!(writer, "</{}>", name)?;
            self.write_line(writer)?;
        }
        Ok(())
    }

    fn call_plugin(
        &mut self,
        node: &KdlNode,
        indent: &str,
        mut writer: Writer,
    ) -> EmitResult<bool> {
        let mut needs_mut_plugin = None;
        for (i, plug) in self.plugins.iter().enumerate() {
            let ctx = PluginContext {
                indent,
                emitter: self,
                writer: &mut writer,
            };
            match plug.0.emit_node(node, ctx)? {
                EmitStatus::Skip => continue,
                EmitStatus::Emmited => return Ok(true),
                EmitStatus::NeedsMutation => {
                    needs_mut_plugin = Some(i);
                    break;
                }
            }
        }
        if let Some(plugin_idx) = needs_mut_plugin {
            // Remove plugin to respect ownership rules
            let mut plugin = self.plugins.remove(plugin_idx);
            let ctx = PluginContext {
                indent,
                emitter: self,
                writer: &mut writer,
            };
            plugin.make_mut().emit_node_mut(node, ctx)?;
            // Reinsert modified plugin
            self.plugins.insert(plugin_idx, plugin);

            return Ok(true);
        }
        Ok(false)
    }

    /// Simply emits the given text content in `content` into the `writer`, indented by the
    /// `indent` param.
    ///
    /// # Example
    /// ```
    /// use kdl::KdlValue;
    /// use htmeta::HtmlEmitter;
    /// let emitter = HtmlEmitter::builder().indent(4).build();
    /// let mut writer = Vec::<u8>::new();
    /// // Usually this value is given to you by other functions.
    /// let indent = emitter.indent();
    /// let value = KdlValue::String("I'm text".into());
    /// emitter.emit_text_node(&indent, &value, &mut writer).unwrap();
    /// assert_eq!(writer, b"I'm text\n");
    /// ```
    pub fn emit_text_node(&self, indent: &str, content: &KdlValue, writer: Writer) -> EmitResult {
        write!(
            writer,
            "{}{}",
            indent,
            html_escape::encode_text(&self.vars.expand_value(content))
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
    /// let doc: KdlDocument = r#"
    ///     html {
    ///         body {
    ///             h1 {
    ///                 text "Title"
    ///             }
    ///         }
    ///     }"#.parse().unwrap();
    /// // Creates an emitter with an indentation level of 4.
    /// let mut emitter = HtmlEmitter::builder().indent(4).build();
    /// // You should wrap this with a `BufWriter` for actual use.
    /// let mut file = std::fs::File::create("index.html").unwrap();
    /// emitter.emit(&doc, &mut file).unwrap();
    /// ```
    pub fn emit<'b: 'a>(&'b mut self, document: &'b KdlDocument, writer: Writer<'b>) -> EmitResult {
        let indent = self.indent();

        for node in document.nodes() {
            let name = node.name().value();

            // variable node
            if name.starts_with("$")
                && let Some(val) = node.get(0)
            {
                let value = self.vars.expand_value(val);
                self.vars.insert(&name[1..], value);
                continue;
            }

            // text/content node
            if (name == "text" || name == "content")
                && let Some(content) = node.get(0)
            {
                self.emit_text_node(&indent, content, writer)?;
                continue;
            }

            // Plugin shenanigans
            if self.call_plugin(node, &indent, writer)? {
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

#[doc(hidden)]
/// This function is used by tests.
/// As to not cause dependency problems, this function is defined here instead
/// of `htmeta-auto-tests`, hence why it is hidden.
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
    auto_html_test!(basic_test2);
    auto_html_test!(basic_var);
    auto_html_test!(var_scopes);

    fn minified() -> HtmlEmitterBuilder {
        let mut builder = HtmlEmitter::builder();
        builder.minify();
        builder
    }

    auto_html_test!(minified_basic, minified());
    auto_html_test!(minified_var_scopes, minified());

    #[derive(Clone)]
    struct ShouterPlugin;

    impl IPlugin for ShouterPlugin {
        fn emit_node(&self, node: &KdlNode, context: PluginContext) -> EmitResult<EmitStatus> {
            let name = node.name().value();
            context
                .emitter
                .emit_tag(node, &name.to_uppercase(), context.indent, context.writer)?;
            Ok(EmitStatus::Emmited)
        }
    }

    fn with_plugin() -> HtmlEmitterBuilder {
        let mut builder = HtmlEmitter::builder();
        builder.add_plugin(ShouterPlugin);
        builder
    }

    auto_html_test!(shouter_basic, with_plugin());
}
