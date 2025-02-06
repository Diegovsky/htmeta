use std::{borrow::Cow, collections::HashMap};

use htmeta::{
    kdl::{KdlDocument, KdlNode},
    EmitResult, EmitStatus, IPlugin, PluginContext,
};

#[derive(Clone, Debug)]
pub struct Template {
    node: KdlNode,
    params: Vec<String>,
}

impl Template {
    fn new(name: &str, node: &KdlNode) -> EmitResult<Self> {
        let mut node = node.clone();
        let Some(children) = node.children_mut() else {
            return Err(format!("{name}: Template tags must have children!"))?;
        };

        let params = match Self::get_special_node(children, "@params") {
            Some(node) => node
                .entries()
                .iter()
                .filter(|e| e.name().is_none())
                .map(|e| e.value().to_string())
                .collect(),
            None => Default::default(),
        };
        Ok(Template {
            node: node.clone(),
            params,
        })
    }
    fn get_special_node(children: &mut KdlDocument, key: &str) -> Option<KdlNode> {
        let i = children
            .nodes()
            .iter()
            .position(|node| node.name().value() == key)?;
        Some(children.nodes_mut().remove(i))
    }
    fn is_property(&self, key: &str) -> bool {
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
        context: PluginContext,
    ) -> EmitResult<EmitStatus> {
        if node.children().is_some() {
            return Err(format!(
                "{name}: Template instantiations must not have bodies!"
            ))?;
        }
        let mut subemitter = context.emitter.clone();

        let templates = &self.templates;
        let Some(template) = templates.get(name) else {
            return Ok(EmitStatus::Skip);
        };
        // Duplicates the node's args into the emitter for instantiation
        // Also turns arguments into $0, $1, etc.
        let mut current_index = 0usize;
        subemitter.vars.extend(node.entries().iter().map(|entry| {
            (
                entry
                    .name()
                    // entry is a property
                    .map(|e| Cow::Borrowed(e.value()))
                    .unwrap_or_else(|| {
                        // entry is an argument, calculate its id
                        let id = current_index;
                        current_index += 1;
                        Cow::Owned(id.to_string())
                    }),
                context.emitter.vars.expand_value(entry.value()),
            )
        }));

        // Creates special variable `props` which contains all unused properties.
        let props = node
            .entries()
            .iter()
            .filter_map(|e| {
                let name = e.name()?;
                if template.is_property(name.value()) {
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
        Ok(EmitStatus::Emmited)
    }
    fn execute_command(
        &mut self,
        name: &str,
        node: &KdlNode,
        context: &PluginContext,
    ) -> EmitResult {
        match name {
            "template" => {
                let template_name = node.get(0).or_else(|| node.get("name")).ok_or_else(|| {
                    format!("{name}: Template tags must have a `name` parameter!")
                })?;
                let template_name = context
                    .emitter
                    .vars
                    .expand_value(template_name)
                    .into_owned();
                let template = Template::new(&template_name, node);
                self.templates.insert(template_name, template?);
            }
            "import" => {
                let template_name = node
                    .get(0)
                    .ok_or_else(|| format!("{name}: Import tags must have path"))?
                    .to_string();
                let template_name = context
                    .emitter
                    .filename
                    .as_ref()
                    .and_then(|original_filename| original_filename.parent())
                    .map(|dir| dir.join(&template_name))
                    .unwrap_or_else(|| template_name.into());
                if !template_name.exists() {
                    return Err(format!(
                        "Failed to find file {}. Original file: {:?}",
                        template_name.display(),
                        context.emitter.filename
                    ))?;
                }
                let doc = std::fs::read_to_string(template_name)?;
                let doc = doc.parse::<KdlDocument>().map_err(|e| e.to_string())?;
                for node in doc.nodes() {
                    if let Some(name) = node.name().value().strip_prefix("@") {
                        // Read commands from the file
                        self.execute_command(name, node, context)?;
                    }
                }
            }
            _ => return Err(format!("Unexpected tag: {name}"))?,
        }
        Ok(())
    }
}

impl IPlugin for TemplatePlugin {
    fn emit_node(&self, node: &KdlNode, context: PluginContext) -> EmitResult<EmitStatus> {
        let name = node.name().value();
        let Some(name) = name.strip_prefix('@') else {
            return Ok(EmitStatus::Skip);
        };
        // Template registry command
        match name {
            "import" | "template" => Ok(EmitStatus::NeedsMutation),
            _ => self.emit_template(name, node, context),
        }
    }
    fn emit_node_mut(&mut self, node: &KdlNode, context: PluginContext) -> EmitResult<()> {
        let name = node.name().value();
        let Some(name) = name.strip_prefix('@') else {
            return Err(format!("Unexpected tag in `emit_node_mut`: {name}"))?;
        };
        self.execute_command(name, node, &context)?;
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
