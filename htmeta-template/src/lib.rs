use std::{borrow::Cow, collections::HashMap};

use htmeta::{
    kdl::{KdlDocument, KdlEntry, KdlNode},
    plugins::{EmitStatus, IPlugin, PluginContext},
    EmitResult, HtmlEmitter,
};

mod utils;
use utils::*;

#[derive(Clone, Debug)]
pub struct Template {
    node: KdlNode,
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
            node: node.clone(),
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
}

impl TemplatePlugin {
    fn emit_template(
        &self,
        name: &str,
        node: &KdlNode,
        context: PluginContext<&HtmlEmitter>,
    ) -> EmitResult {
        if node.children().is_some() {
            return Err(format!(
                "{name}: Template instantiations must not have bodies!"
            ))?;
        }
        let mut subemitter = context.emitter.clone();

        let template = &self.templates[name];

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

        subemitter.emit(
            template
                .node
                .children()
                .expect("Internal error: template tags must have children"),
            context.writer,
        )?;
        Ok(())
    }
    // fn execute_command(
    //     &self,
    //     command: &str,
    //     node: &KdlNode,
    //     context: &PluginContext,
    // ) -> EmitResult {
    //     match command {
    //         _ => return Err(format!("Unexpected tag: {command}"))?,
    //     }
    //     Ok(())
    // }
    fn execute_command_mut(
        &mut self,
        command: &str,
        node: &KdlNode,
        context: &mut PluginContext<&mut HtmlEmitter>,
    ) -> EmitResult {
        match command {
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
                let filename = node
                    .get(0)
                    .ok_or_else(|| format!("{command}: Import tags must have path"))?
                    .to_string();
                let filename = context
                    .emitter
                    .filename
                    .as_ref()
                    .and_then(|original_filename| original_filename.parent())
                    .map(|dir| dir.join(&filename))
                    .unwrap_or_else(|| filename.into());
                if !filename.exists() {
                    return Err(format!(
                        "Failed to find file {}. Original file: {:?}",
                        filename.display(),
                        context.emitter.filename
                    ))?;
                }
                let doc = std::fs::read_to_string(filename)?;
                let doc = doc.parse::<KdlDocument>().map_err(|e| e.to_string())?;
                if command == IMPORT {
                    // Executes commands from the file, but keeping scope intact.
                    for node in doc.nodes() {
                        if let Some(name) = node.name().value().strip_prefix("@") {
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
            Some(DBG) => EmitStatus::Emit,
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
