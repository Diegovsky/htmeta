#![feature(let_chains)]
/// ![GitHub Release](https://img.shields.io/github/v/release/Diegovsky/htmeta)
/// ![GitHub Repo stars](https://img.shields.io/github/stars/Diegovsky/htmeta)
/// ![GitHub Forks](https://img.shields.io/github/forks/Diegovsky/htmeta)
/// ![GitHub Contributors](https://img.shields.io/github/contributors/Diegovsky/htmeta)
///
/// This crate allows you to transform/transpile/compile a [`KDL`] document into `HTML`.
/// Since the `kdl` dependency is unavoidable, it is re-exported for convenience as [`kdl`].
///
/// # Basic Example
/// The following function can be used to turn `htmeta` strings into `HTML`:
/// ```
/// use htmeta::{HtmlEmitter};
/// use htmeta::kdl;
/// use std::path::PathBuf;
/// fn emit_str(text: &str) -> String {
///     let doc = text.parse::<kdl::KdlDocument>().unwrap();
///     let builder = HtmlEmitter::builder();
///     let mut emitter = builder.build(PathBuf::from("<string>"));
///     let mut buf = Vec::new();
///     emitter.emit(&doc, &mut buf).unwrap();
///     String::from_utf8(buf).unwrap()
/// }
///
/// assert_eq!(emit_str(r#"body { p "Hi!"  }"#), r#"<body>
///     <p>Hi!</p>
/// </body>
/// "#);
/// ```
///
/// [`KDL`]: https://kdl.dev

macro_rules! re {
    ($name:ident, $e:expr) => {
        use regex::Regex;
        use std::sync::LazyLock;
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($e).unwrap());
    };
}

use std::{borrow::Cow, collections::HashMap, io::Write, path::PathBuf, rc::Rc};

use dyn_clone::DynClone;
/// A helpful re-export to our `kdl` library.
pub use kdl;

use kdl::{KdlDocument, KdlNode, KdlValue};
use plugins::{EmitStatus, IPlugin, Plugin, PluginContext};
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

pub type Text<'b> = Cow<'b, str>;

mod error;
pub mod plugins;
pub mod utils;

pub use error::Error;

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr", "!DOCTYPE", "!doctype", // not a tag at all, but works a lot like one.
];

/// A builder for [`HtmlEmitter`]s.
#[derive(Clone, Default)]
pub struct HtmlEmitterBuilder {
    indent: Option<Indent>,
    plugins: Vec<Plugin>,
}

impl HtmlEmitterBuilder {
    /// Returns a new [`Self`] instance. By default, each node is indented by `4` spaces.
    /// To override the amount, check out [`Self::indent`].
    pub fn new() -> Self {
        Self {
            indent: Some(4),
            ..Self::default()
        }
    }

    /// Makes the indentation level follow the original document's for each node.
    /// This is currently experimental.
    pub fn follow_original_indent(&mut self) -> &mut Self {
        self.indent = None;
        self
    }

    /// Overrides the document indentation. That is, it always indentates
    /// child nodes by `indent` spaces.
    pub fn indent(&mut self, indent: Indent) -> &mut Self {
        self.indent = indent.into();
        self
    }

    /// Disables indentation and newlines.
    pub fn minify(&mut self) -> &mut Self {
        self.indent = 0.into();
        self
    }

    /// Registers a plugin for all instances of this builder.
    pub fn add_plugin<P: IPlugin + 'static>(&mut self, plugin: P) -> &mut Self {
        self.plugins.push(Plugin::new(plugin));
        self
    }

    /// Creates a new [`HtmlEmitter`]. You should re-use this builder to create emitters
    /// efficiently.
    pub fn build<'a>(&self, filename: impl Into<Option<PathBuf>>) -> HtmlEmitter<'a> {
        HtmlEmitter {
            current_level: 0,
            indent: self.indent,
            plugins: self.plugins.clone().into(),
            vars: Default::default(),
            filename: filename.into().map(|f| Rc::new(f)),
        }
    }
}

type VarMap<'content> = HashMap<Box<str>, Text<'content>>;

/// Holds all node's variables
#[derive(Clone, Debug, Default)]
pub struct Vars<'content> {
    vars: Rc<VarMap<'content>>,
}

impl<'content> Vars<'content> {
    pub fn new(map: VarMap<'content>) -> Self {
        Self { vars: map.into() }
    }
    /// Replaces all occurences of variables inside `text` and returns a new string.
    pub fn expand_string<'b>(&self, text: &'b str) -> Text<'b> {
        re!(VAR, r"\$(\$|\w+)");
        VAR.replace_all(text, |captures: &Captures| {
            let capture = &captures[1];
            if capture == "$" {
                return "$"
            }
            let var = self.vars
                .get(capture)
                .map(Cow::as_ref)
                .unwrap_or_default();
            var
        })
    }

    /// Converts the `value`'s [`String`] representation and replaces any variables found within.
    /// This is a convenient wrapper around [`Self::expand_string`].
    pub fn expand_value<'b>(&self, value: &'b KdlValue) -> Text<'b> {
        match value {
            KdlValue::String(content) => self.expand_string(content),
            rest => rest.to_string().into(),
        }
    }

    fn make_mut(&mut self) -> &mut VarMap<'content> {
        Rc::make_mut(&mut self.vars)
    }

    /// Inserts a new variable into the node.
    pub fn insert(&mut self, key: &str, value: Text<'content>) {
        self.make_mut().insert(key.into(), value);
    }

    /// Returns a reference to a variable's value.
    pub fn get(&self, key: &str) -> Option<&Text<'content>> {
        self.vars.get(key)
    }

    /// Clears the node, removing all registered variables.
    pub fn clear(&mut self) {
        self.make_mut().clear();
    }

    /// Returns an iterator of Key-Value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Box<str>, &Text<'content>)> {
        self.vars.iter()
    }

    fn unwrap(self) -> VarMap<'content> {
        Rc::try_unwrap(self.vars).unwrap_or_else(|rc| (*rc).clone())
    }

    /// Turns all borrowed content into owned content
    pub fn into_owned(self) -> Vars<'static> {
        Vars::new(
            self.unwrap()
                .into_iter()
                .map(|(k, v)| (k, v.into_owned().into()))
                .collect(),
        )
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

impl<'a> std::iter::IntoIterator for Vars<'a> {
    type Item = (Box<str>, Text<'a>);
    type IntoIter = std::collections::hash_map::IntoIter<Box<str>, Text<'a>>;
    fn into_iter(self) -> Self::IntoIter {
        self.unwrap().into_iter()
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
///         h1 "Title"
///
///     }
/// }"#.parse().unwrap();
///
/// // Creates an emitter with an indentation level of 4.
/// let mut emitter = HtmlEmitter::builder().indent(4).build(Some(Default::default()));
///
/// // Emits html to the terminal.
/// emitter.emit(&doc, &mut std::io::stdout()).unwrap();
/// ```
#[derive(Clone)]
pub struct HtmlEmitter<'a> {
    /// When fixed indentation is enabled, contains the amount of space
    /// characters corresponding to one indentaion level.
    pub indent: Option<Indent>,
    /// Contains the depth of this emmiter, that is, how deep it is compared
    /// to a root node
    pub current_level: Indent,
    /// Contains a node's variables.
    pub vars: Vars<'a>,

    pub filename: Option<Rc<PathBuf>>,
    /// The current list of plugins
    pub plugins: im_rc::Vector<Plugin>,
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

    /// Returns `true` if in minify mode, `false` otherwise.
    pub fn is_minify(&self) -> bool {
        self.indent == Some(0)
    }

    /// Convenience function that writes a newline if not in `minify` mode.
    pub fn write_line(&self, writer: Writer) -> EmitResult {
        if !self.is_minify() {
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
    /// let emitter = HtmlEmitter::builder().indent(4).build(Some(Default::default()));
    /// assert_eq!(emitter.indent(&htmeta::kdl::KdlNode::new("")), "");
    /// ```
    pub fn indent(&self, node: &KdlNode) -> String {
        match self.indent {
            Some(indent) => " ".repeat(self.current_level * indent),
            None => node
                .format()
                .map(|fmt| fmt.leading.clone())
                .unwrap_or_default(),
        }
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
    /// let emitter = HtmlEmitter::builder().minify().build(Some(Default::default()));
    /// // Creates a simple paragraph node
    /// let node = r#"p id="paragraph" "Hello, world!""#.parse::<KdlNode>().unwrap();
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

        if is_void && node.children().is_some() {
            return Err("Void tags can't have children")?;
        }

        // opening tag
        write!(writer, "{}<{}", indent, name)?;

        let mut entries = node.entries().to_vec();

        let mut contents = None;
        // If the last entry is a string arg, the node has no children and it isn't a VOID tag, use it as contents.
        if !is_void
            && node.children().is_none()
            && matches!(entries.last(), Some(entry) if entry.name().is_none())
        {
            let entry = entries.remove(entries.len() - 1);
            contents = Some(entry);
        }

        // args
        for entry in entries {  //
            let value = self.vars.expand_value(entry.value());
            if value.is_empty() {
                continue;
            }
            write!(writer, " ")?;
            if let Some(name) = entry.name() {
                let name = self.vars.expand_string(name.value());
                write!(writer, "{name}=\"{value}\"")?;
            } else {
                write!(writer, "{}", value)?;
            }
        }

        if is_void {
            write!(writer, ">")?;
            self.write_line(writer)?;
        } else {
            write!(writer, ">")?;
            if let Some(contents) = contents {
                // If node has children and text, print each in their own line
                write!(writer, "{}", self.vars.expand_value(contents.value()))?;
            }
            // Children
            else if let Some(doc) = node.children() {
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

    fn call_plugin<'b>(
        &'b mut self,
        node: &'b KdlNode,
        indent: &'b str,
        mut writer: Writer<'_>,
    ) -> EmitResult<bool> {
        for plug in self.plugins.clone().iter() {
            match plug.should_emit(node, self) {
                EmitStatus::Skip => continue,
                EmitStatus::Emit => {
                    plug.emit_node(
                        node,
                        PluginContext {
                            indent,
                            writer: &mut writer,
                            emitter: self,
                        },
                    )?;
                    return Ok(true);
                }
            }
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
    /// let emitter = HtmlEmitter::builder().indent(4).build(Some(Default::default()));
    /// let mut writer = Vec::<u8>::new();
    /// // Usually this value is given to you by other functions.
    /// let indent = ""; // no indentation
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

    /// Converts `content` into an unquoted string and writes its contents directly to the `writer`,
    /// without any escaping.
    ///
    /// Note that $variables are still expanded.
    pub fn emit_raw_text(&self, indent: &str, content: &KdlValue, writer: Writer) -> EmitResult {
        write!(writer, "{}{}", indent, &self.vars.expand_value(content))?;
        self.write_line(writer)?;
        Ok(())
    }

    pub fn variable_node<'b: 'a>(&mut self, name: &str, node: &'b KdlNode) -> bool {
        if name.starts_with("$")
            && let Some(val) = node.get(0)
        {
            let value = self.vars.expand_value(val);
            self.vars.insert(&name[1..], value);
            return true;
        }
        false
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
    ///             h1 "Title"
    ///         }
    ///     }"#.parse().unwrap();
    /// // Creates an emitter with an indentation level of 4.
    /// let mut emitter = HtmlEmitter::builder().indent(4).build(Some(Default::default()));
    /// // You should wrap this with a `BufWriter` for actual use.
    /// let mut file = std::fs::File::create("index.html").unwrap();
    /// emitter.emit(&doc, &mut file).unwrap();
    /// ```
    pub fn emit<'b: 'a>(&mut self, document: &'b KdlDocument, writer: Writer<'b>) -> EmitResult {
        for node in document.nodes() {
            let name = node.name().value();
            let indent = self.indent(node);

            // variable node
            if self.variable_node(name, node) {
                continue;
            }

            if name == "_"
                && let Some(content) = node.get(0)
            {
                self.emit_raw_text(&indent, content, writer)?;
                continue;
            }

            // text/content node
            if (name == "-" || name == "text")
                && let Some(content) = node.get(0)
            {
                if name == "text" {
                    eprintln!("`text` nodes are now deprecated. Please use the new syntax.\n")
                }
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
        Ok(())
    }

    /// Clears all variables and resets plugins, allowing this instance to be reused.
    pub fn clear(&mut self) {
        self.vars.clear();
        for plugin in self.plugins.iter_mut() {
            plugin.make_mut().clear()
        }
    }

    /// Clones borrowed data from [`KdlNode`], removing lifetime bounds.
    pub fn into_owned(self) -> HtmlEmitter<'static> {
        let Self {
            indent,
            current_level,
            vars,
            filename,
            plugins,
        } = self;
        let vars: Vars<'static> = vars.into_owned();
        HtmlEmitter {
            indent,
            current_level,
            vars,
            filename,
            plugins,
        }
    }
}

#[doc(hidden)]
/// This function is used by tests.
/// As to not cause dependency problems, this function is defined here instead
/// of `htmeta-auto-tests`, hence why it is hidden.
pub fn emit_as_str(builder: &HtmlEmitterBuilder, input: &str) -> EmitResult<String> {
    let doc: kdl::KdlDocument = input.parse().expect("Failed to parse as kdl doc");
    let mut buf = Vec::<u8>::new();
    let mut emitter = builder.build(PathBuf::from("<string>"));
    emitter.emit(&doc, &mut buf)?;
    Ok(String::from_utf8(buf).expect("Invalid utf8 found"))
}

#[cfg(test)]
pub mod tests;
