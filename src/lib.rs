#![feature(let_chains)]
//! This crates implements a (simple) flavour of [`KDL`] called `htmeta`. This dialect's purpose is
//! to allow a straightforward representation of `HTML`.
//!
//! # Format
//! As [`KDL`] is already very similar to `HTML` semantically, `htmeta` only adds 2 things:
//!  - A way to differentiate true `text` content to be shown in `HTML`.
//!  - Variables to reduce repetition.
//!
//! ## Text nodes
//! Text nodes are creatively named `text` and they can only have one positional argument, which is
//! the text to be directly pasted into the resulting `HTML`.
//! 
//! Example:
//! ```kdl
//! html {
//!     body {
//!         h1 {
//!             text "Title"
//!         }
//!     }
//! }
//! ```
//!
//! Results in:
//! ```html
//! <html>
//!     <body>
//!         <h1>
//!             Title
//!         </h1>
//!     </body>
//! </html>
//! ```
//!
//! ## Variables
//! If you ever used CSS-based frameworks like `TailwindCSS` or `Bootstrap`, you know
//! how tedious it is to type the same classes over and over again. Hence, `htmeta` implements a
//! simple variable mechanism that reduces duplication.
//!
//! Example:
//! ```kdl
//! html {
//!     head {
//!         meta charset="utf-8"
//!         // Includes tailwindcss
//!         script src="https://cdn.tailwindcss.com"
//!     }
//!     body {
//!         // creates a variable called `$btn_class`
//!         $btn_class "border-1 rounded-lg"
//!
//!         // Value of `$btn_class` is reused inside these buttons:
//!         button class="$btn_class ml-4"
//!         bttton class="$btn_class mr-4"
//!     }
//! }
//! ```
//!
//! Results in:
//! ```html
//! <html>
//!     <head>
//!         <meta charset="utf-8"/>
//!         <script src="https://cdn.tailwindcss.com"></script>
//!     </head>
//!     <body>
//!         <button class="border-1 rounded-lg ml-4"></button>
//!         <bttton class="border-1 rounded-lg mr-4"></bttton>
//!     </body>
//! </html>
//! ```
//!
//! [`KDL`]: https://kdl.dev/

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
    io::{self, Write},
};

use kdl::{KdlDocument, KdlNode, KdlValue};
use regex::Captures;

type Writer<'a> = &'a mut dyn Write;
type Text<'b> = Cow<'b, str>;
type EmitResult<T = ()> = io::Result<T>;

/// An `HTML` emitter for `htmeta`.
///
/// ```rust
/// use kdl::KdlDocument;
/// let doc = KdlDocument::from_str(r#"html { body { h1 { text "Title" }}}"#).unwrap();
///
/// // Creates an emitter with an indentation level of 4.
/// let emitter = HtmlEmitter::new(&doc, 4);
/// ```
#[derive(Clone)]
pub struct HtmlEmitter<'a> {
    node: &'a KdlDocument,
    indent: usize,
    current_level: usize,
    vars: HashMap<&'a str, Text<'a>>,
}

fn get_attribute<'a>(node: &'a KdlNode, name: &str) -> Option<&'a KdlValue> {
    if let Some(entry) = node.get(name) {
        return Some(entry.value());
    }
    node.children().and_then(|node| node.get_arg(name))
}

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

impl<'a> HtmlEmitter<'a> {
    /// Creates a new [`HtmlEmitter`] with an indentation level of `indent`.
    pub fn new(node: &'a KdlDocument, indent: usize) -> Self {
        Self {
            node,
            indent,
            current_level: 0,
            vars: Default::default(),
        }
    }

    fn subemitter(&self, node: &'a KdlDocument) -> Self {
        Self {
            current_level: self.current_level + 1,
            node,
            ..self.clone()
        }
    }

    fn indent(&mut self) -> String {
        " ".repeat(self.current_level * self.indent)
    }

    fn expand_string<'b>(&self, text: &'b str) -> Text<'b> {
        re!(VAR, r"(\$\w+)");
        VAR.replace(text, |captures: &Captures| {
            self.vars
                .get(&captures[1])
                .map(ToString::to_string)
                .unwrap_or_default()
        })
    }

    fn expand_value<'b>(&self, value: &'b KdlValue) -> Text<'b> {
        match value {
            KdlValue::RawString(content) | KdlValue::String(content) => self.expand_string(content),
            _ => todo!(),
        }
    }

    fn emit_tag(
        &mut self,
        node: &'a KdlNode,
        name: &str,
        indent: &str,
        writer: &mut Writer<'a>,
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
            writeln!(writer, "/>")?;
        } else {
            write!(writer, ">")?;
            // Children
            if let Some(doc) = node.children() {
                writeln!(writer)?;
                self.subemitter(doc).emit(writer)?;
                write!(writer, "{}", indent)?;
            }
            writeln!(writer, "</{}>", name)?;
        }
        Ok(())
    }

    /// Emits the corresponding `HTML` into the `writer`. The emitter can be re-used after this.
    ///
    /// # Examples:
    ///
    /// ```rust
    /// use kdl::KdlDocument;
    /// let doc = KdlDocument::from_str(r#"html { body { h1 { text "Title" }}}"#).unwrap();
    /// // Creates an emitter with an indentation level of 4.
    /// let emitter = HtmlEmitter::new(&doc, 4);
    /// // You should wrap this with a `BufWriter` for actual use.
    /// let file = std::fs::create("index.html").unwrap();
    /// emitter.emit(&mut file).unwrap();
    /// ```
    pub fn emit(&mut self, mut writer: Writer<'a>) -> EmitResult {
        let indent = self.indent();

        for node in self.node.nodes() {
            let name = node.name().value();

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
                writeln!(
                    writer,
                    "{}{}",
                    indent,
                    html_escape::encode_text(&self.expand_value(content.value()))
                )?;
                continue;
            }

            self.emit_tag(node, name, &indent, &mut writer)?
        }
        // Allows this instance to be reused
        self.vars.clear();
        Ok(())
    }
}
