use std::collections::HashMap;

use htmeta::{
    kdl::KdlNode, EmitResult, EmitStatus, IPlugin, PluginContext
};

#[derive(Debug, Default, Clone)]
pub struct TemplatePlugin {
    templates: HashMap<String, KdlNode>,
}

impl TemplatePlugin {
    fn emit_template(
        &self,
        name: &str,
        node: &KdlNode,
        context: PluginContext,
    ) -> EmitResult<EmitStatus> {
        if node.children().is_some() {
            return Err(format!("{name}: Template instantiations must not have bodies!"))?
        }
        let mut subemitter = context.emitter.clone();

        let templates = &self.templates;
        let Some(template) = templates.get(name) else {
            return Ok(EmitStatus::Skip);
        };
        subemitter
            .vars
            .extend(node.entries().iter().filter_map(|entry| {
                Some((
                    entry.name()?.value(),
                    context.emitter.vars.expand_value(entry.value()),
                ))
            }));
        subemitter.emit(template.children().expect("Internal error: template tags must have children"), context.writer)?;
        Ok(EmitStatus::Emmited)
    }
}

impl IPlugin for TemplatePlugin {
    fn emit_node(&self, node: &KdlNode, context: PluginContext) -> EmitResult<EmitStatus> {
        let name = node.name().value();
        let Some(name) = name.strip_prefix('@') else {
            return Ok(EmitStatus::Skip);
        };
        // Template registry command
        if name == "template" {
            Ok(EmitStatus::NeedsMutation)
        } else {
            self.emit_template(name, node, context)
        }
    }
    fn emit_node_mut(&mut self, node: &KdlNode, context: PluginContext) -> EmitResult<()> {
        let name = node.name().value();
        let Some(name) = name.strip_prefix('@') else {
            return Err(format!("Unexpected tag in `emit_node_mut`: {name}"))?
        };
        let template_name = node
            .get("name")
            .ok_or_else(|| format!("{name}: Template tags must have a `name` parameter!"))?;
        if node.children().is_none() {
            return Err(format!("{name}: Template tags must have children!"))?;
        }
        self.templates.insert(
            context
                .emitter
                .vars
                .expand_value(template_name)
                .into_owned(),
            node.clone(),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use htmeta::{HtmlEmitter, HtmlEmitterBuilder};
    use htmeta_auto_test::*;
    use htmeta::emit_as_str;

    fn builder() -> HtmlEmitterBuilder {
        let mut builder = HtmlEmitter::builder();
        builder.add_plugin(TemplatePlugin::default());
        builder
    }

    auto_html_test!(basic_test, builder());
    auto_html_test!(param_test, builder());
    auto_html_test!(param_compose_test, builder());
}
