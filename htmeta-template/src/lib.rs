use std::{borrow::Cow, collections::{HashMap, HashSet}, path::{Path, PathBuf}, rc::Rc};

use htmeta::{
    EmitResult, HtmlEmitter,
    kdl::{KdlDocument, KdlEntry, KdlNode},
    plugins::{EmitStatus, IPlugin, PluginContext},
};

mod utils;
use utils::*;

#[derive(Clone, Debug)]
pub struct Template {
    node: KdlNode,
    uses_children: bool,
    params: Vec<String>,
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
            Some(node) => node.args().map(|e| e.to_string()).collect(),
            None => Default::default(),
        };
        Ok(Template {
            uses_children: find(&node, &|node| node.is_command("children")),
            node,
            params,
        })
    }
    fn is_param(&self, key: &str) -> bool {
        self.params.iter().find(|el| *el == key).is_some()
    }
}

#[derive(Debug, Default, Clone)]
pub struct TemplatePlugin {
    templates: HashMap<String, Template>,
    file_dep_graph: HashMap<Rc<PathBuf>, HashSet<Rc<PathBuf>>>
}

impl TemplatePlugin {
    pub fn used_files(&self) -> impl Iterator<Item = &Path> {
        self.file_dep_graph.keys().map(|i| i.as_path())
    }
    fn emit_template(
        &self,
        name: &str,
        node: &KdlNode,
        context: PluginContext<&HtmlEmitter>,
    ) -> EmitResult {
        let mut subemitter = context.emitter.clone();

        let template = &self.templates[name];

        if node.children().is_some() && !template.uses_children {
            return Err(format!(
                "{name}: Template was called with children but does not support it!"
            ))?;
        }
        if find(node, &|node| node.is_command("children")) {
            return Err(format!(
                "{name}: Template children contain @children. Infinite recursion detected."
            ))?;

        }
        let template_children = node.iter_children().cloned().collect::<Vec<_>>();

        // Duplicates the node's args into the emitter for instantiation
        // Also turns arguments into $0, $1, etc.
        subemitter
            .vars
            .extend(node.keyed_entries().map(|(key, value)| {
                (
                    Cow::<str>::from(key),
                    context.emitter.vars.expand_value(value),
                )
            }));

        // Creates special variable `props` which contains all unused properties.
        let props = node
            .entries()
            .iter()
            .filter_map(|e| {
                let name = e.name()?;
                if template.is_param(name.value()) {
                    None
                } else {
                    Some(e.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        subemitter.vars.insert("props", props.into());

        let mut template_node = template.node.clone();
        // recursively replace @children with the children block.
        for_each_mut(&mut template_node, &|node| {
            let Some(children) = node.children_mut().as_mut() else {return};
            let children = children.nodes_mut();
            *children = std::mem::take(children).into_iter().flat_map(|c| {
                if c.is_command("children") {
                    template_children.clone()
                } else {
                    vec![c]
                }
            }).collect();
        });

        subemitter.emit(
            template_node
                .children()
                .expect("Internal error: template tags must have children"),
            context.writer,
        )?;
        Ok(())
    }
    fn add_path(&mut self, filename: Rc<PathBuf>) -> &mut HashSet<Rc<PathBuf>> {
        self.file_dep_graph.entry(filename).or_default()
    }
    fn execute_command_mut(
        &mut self,
        command: &str,
        node: &KdlNode,
        context: &mut PluginContext<&mut HtmlEmitter>,
    ) -> EmitResult {
        match command {
            "children" => {
                return Err(format!("@children is a reserved name."))?;
            },
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
                self.templates.insert(template_name, template?);
            }
            // reads a file and executes commands
            IMPORT | INCLUDE => {
                let include_path = node
                    .get(0)
                    .ok_or_else(|| format!("{command}: Import tags must have path"))?
                    .as_string().ok_or_else(|| format!("Import tags must only receive strings"))?;

                let current_filename: Rc<PathBuf> = context
                    .emitter
                    .filename
                    .clone()
                    .unwrap_or_default();

                let current_dirname = current_filename.parent().unwrap_or(Path::new("."));
                let filename = current_dirname.join(context.emitter.vars.expand_string(include_path).as_ref());
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
                if command == IMPORT {
                    // Executes commands from the file, but keeping scope intact.
                    for node in doc.nodes() {
                        if let Some(name) = node.command_name() {
                            // Read commands from the file
                            self.execute_command_mut(name, node, context)?;
                        }
                    }
                } else {
                    // Straight up emits the file in-place
                    let mut em = context.emitter.clone();
                    em.emit(&doc, context.writer)?;
                    // Copies variables into the current context;
                    let vars = em.vars.into_owned();
                    context.emitter.vars.extend(vars);
                }
            }
            _ => return Err(format!("Unexpected tag: {command}"))?,
        }
        Ok(())
    }
}

impl IPlugin for TemplatePlugin {
    fn should_emit(&self, node: &KdlNode, emitter: &HtmlEmitter) -> EmitStatus {
        let _ = emitter;
        match node.command_name() {
            None => EmitStatus::Skip,
            Some(IMPORT | TEMPLATE | INCLUDE) => EmitStatus::EmitMut,
            Some(DBG | FOR) => EmitStatus::Emit,
            Some(template_name) => {
                if self.templates.contains_key(template_name) {
                    EmitStatus::Emit
                } else {
                    EmitStatus::Skip
                }
            }
        }
    }
    fn emit_node(&self, node: &KdlNode, mut context: PluginContext<&HtmlEmitter>) -> EmitResult {
        match node.command_name().unwrap() {
            DBG => {
                let mut node = KdlNode::new("code");
                node.entries_mut()
                    .push(KdlEntry::new(format!("{:#?}", context.emitter.vars)));
                context
                    .emitter
                    .emit_tag(&node, "code", context.indent, &mut context.writer)
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

                for value in args.into_iter().rev() {
                    let mut emit = context.emitter.clone();
                    emit.vars.insert(name, emit.vars.expand_value(value));
                    emit.emit(children, context.writer)?;
                }
                Ok(())
            }
            name => self.emit_template(name, node, context),
        }
    }
    fn emit_node_mut(
        &mut self,
        node: &KdlNode,
        mut context: PluginContext<&mut HtmlEmitter>,
    ) -> EmitResult {
        let Some(name) = node.command_name() else {
            return Err(format!(
                "Unexpected tag in `emit_node_mut`: {}",
                node.name().value()
            ))?;
        };
        self.execute_command_mut(name, node, &mut context)?;
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
