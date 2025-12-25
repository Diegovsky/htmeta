use std::{
    borrow::Cow,
    cell::{RefCell, RefMut},
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
};

use htmeta::{
    EmitResult, HtmlEmitter,
    kdl::{KdlDocument, KdlEntry, KdlIdentifier, KdlNode, KdlValue},
    plugins::{EmitStatus, IPlugin, PluginContext},
    utils::NilWriter,
};

mod expr;
mod utils;
use utils::*;

use crate::expr::parse_range;

#[derive(Clone, Debug)]
pub struct Template {
    node: KdlNode,
    uses_children: bool,
    params: HashMap<String, Option<KdlValue>>,
}

macro_rules! cmds {
    ($($name:ident = $val:expr);* $(;)?) => {
        $(
            pub const $name: &str = $val;
        )*

        // pub const COMMANDS: &[&str] = &[$($name),*];
    };
}

cmds! {
    IMPORT = "import";
    TEMPLATE = "template";
    INCLUDE = "include";
    DBG = "dbg";
    FOR = "for";
    CHILDREN = "children";
}

fn find(node: &KdlNode, filter: &impl Fn(&KdlNode) -> bool) -> bool {
    filter(node) || node.iter_children().any(|n| find(n, filter))
}

fn for_each_mut(node: &mut KdlNode, map: &impl Fn(&mut KdlNode)) {
    map(node);
    node.iter_children_mut()
        .for_each(|node| for_each_mut(node, map));
}

impl Template {
    fn new(name: &str, node: &KdlNode) -> EmitResult<Self> {
        let mut node = node.clone();
        let Some(children) = node.children_mut() else {
            return Err(format!("{name}: Template tags must have children!"))?;
        };

        let params = match children.remove_child("@params") {
            Some(node) => node
                .entries()
                .iter()
                .map(|e| {
                    if let Some(name) = e.name() {
                        (name.value().to_owned(), Some(e.value().clone()))
                    } else {
                        (e.to_string(), None)
                    }
                })
                .collect(),
            None => Default::default(),
        };
        Ok(Template {
            uses_children: find(&node, &|node| node.is_command("children")),
            node,
            params,
        })
    }
    fn default_params(&self) -> impl Iterator<Item = (&str, &KdlValue)> {
        self.params
            .iter()
            .filter_map(|(key, val)| Some((key.as_ref(), val.as_ref()?)))
    }
    fn is_param(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }
}

#[derive(Debug, Default, Clone)]
pub struct TemplatePlugin {
    templates: RefCell<HashMap<String, Template>>,
    file_dep_graph: RefCell<HashMap<Rc<PathBuf>, HashSet<Rc<PathBuf>>>>,
}

mod template_instantiation;
impl TemplatePlugin {
    pub fn used_files(&self) -> Vec<Rc<PathBuf>> {
        self.file_dep_graph.borrow().keys().cloned().collect()
    }
    fn add_path(&self, filename: Rc<PathBuf>) -> RefMut<HashSet<Rc<PathBuf>>> {
        RefMut::map(self.file_dep_graph.borrow_mut(), |it| {
            it.entry(filename).or_default()
        })
    }
}

impl IPlugin for TemplatePlugin {
    fn should_emit(&self, node: &KdlNode, emitter: &HtmlEmitter) -> EmitStatus {
        let _ = emitter;
        match node.command_name() {
            None => EmitStatus::Skip,
            Some(IMPORT | TEMPLATE | INCLUDE | DBG | FOR) => EmitStatus::Emit,
            Some(template_name) => {
                if self.templates.borrow().contains_key(template_name) {
                    EmitStatus::Emit
                } else {
                    EmitStatus::Skip
                }
            }
        }
    }
    fn emit_node(&self, node: &KdlNode, mut context: PluginContext) -> EmitResult {
        let command = node.command_name().unwrap();
        match command {
            DBG => {
                let code = format!("<code>{:#?}</code>", context.emitter.vars);
                let mut pre = KdlNode::new("pre");
                pre.entries_mut().push(KdlEntry::new(code));
                context
                    .emitter
                    .emit_tag(&pre, "pre", context.indent, &mut context.writer)?;
            }
            FOR => {
                let mut args = node.args().rev().collect::<Vec<_>>();
                let name = args
                    .pop()
                    .ok_or_else(|| err("for: can't iterate without binding name"))?
                    .as_string()
                    .ok_or_else(|| err("for: expected binding name to be a String"))?;

                if args.pop().and_then(|i| i.as_string()) != Some("in") {
                    return Err(err("for: expected `in` keyword after binding name"));
                }
                let children = node
                    .children()
                    .ok_or_else(|| err("for: expected `for` node to have children"))?;

                args.reverse();

                let iter: Box<dyn Iterator<Item = _>> = if let Some(iter) = parse_range(&*args) {
                    Box::new(iter.map(|i| KdlValue::Integer(i as _)).map(Cow::Owned))
                } else {
                    Box::new(args.into_iter().map(Cow::Borrowed))
                };

                for value in iter {
                    let mut emit = context.emitter.clone();
                    emit.vars.insert(name, emit.vars.expand_value(&*value));
                    emit.emit(children, context.writer)?;
                }
            }

            "children" => {
                return Err(format!("@children is a reserved name."))?;
            }
            // registers a template
            TEMPLATE => {
                let template_name = node.get(0).or_else(|| node.get("name")).ok_or_else(|| {
                    format!("{command}: Template tags must have a `name` parameter!")
                })?;
                let template_name = context
                    .emitter
                    .vars
                    .expand_value(template_name)
                    .into_owned();
                let template = Template::new(&template_name, node);
                self.templates.borrow_mut().insert(template_name, template?);
            }
            // reads a file and executes commands
            IMPORT | INCLUDE => {
                let include_path = node
                    .get(0)
                    .ok_or_else(|| format!("{command}: Import tags must have path"))?
                    .as_string()
                    .ok_or_else(|| format!("Import tags must only receive strings"))?;

                let current_filename: Rc<PathBuf> =
                    context.emitter.filename.clone().unwrap_or_default();

                let current_dirname = current_filename.parent().unwrap_or(Path::new("."));
                let filename =
                    current_dirname.join(context.emitter.vars.expand_string(include_path).as_ref());
                if !filename.exists() {
                    return Err(format!(
                        "Failed to find file '{}'. Original file: {:?}",
                        filename.display(),
                        context.emitter.filename
                    ))?;
                }

                // Register dependency on `filename`
                let filename = Rc::new(filename);
                self.add_path(current_filename).insert(filename.clone());
                self.add_path(filename.clone());

                let doc = std::fs::read_to_string(&**filename)?;
                let doc = doc.parse::<KdlDocument>().map_err(|e| e.to_string())?;

                let mut em = context.emitter.clone();
                // let mut em = context.emitter.clone();
                if command == INCLUDE {
                    em.emit(&doc, context.writer)?;
                } else {
                    em.emit(&doc, NilWriter::new())?;
                }
                *context.emitter = em.into_owned();
            }
            name => self.emit_template(name, node, context)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use htmeta::emit_as_str;
    use htmeta::{HtmlEmitter, HtmlEmitterBuilder};
    use htmeta_auto_test::*;

    fn builder() -> HtmlEmitterBuilder {
        let mut builder = HtmlEmitter::builder();
        builder.add_plugin(TemplatePlugin::default());
        builder
    }

    auto_html_test!(basic_test, builder());
    auto_html_test!(param_test, builder());
    auto_html_test!(param_compose_test, builder());
}
