use std::collections::HashMap;

use htmeta::{
    kdl::KdlNode,
    EmitResult, IPlugin, PluginContext,
};
use maybe_sync::Mutex;

#[derive(Debug, Default)]
pub struct TemplatePlugin {
    templates: Mutex<HashMap<String, KdlNode>>,
}

impl Clone for TemplatePlugin {
    fn clone(&self) -> Self {
        Self { templates: Mutex::new(self.templates.lock().clone()) }
    }
}

impl TemplatePlugin {
    fn emit_template(
        &self,
        name: &str,
        node: &KdlNode,
        context: PluginContext,
    ) -> EmitResult<bool> {
        if node.children().is_some() {
            return Err(format!("{name}: Template instantiations must not have bodies!"))?
        }
        let mut subemitter = context.emitter.clone();

        let templates = self.templates.lock();
        let Some(template) = templates.get(name) else {
            return Ok(false);
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
        Ok(true)
    }
}

impl IPlugin for TemplatePlugin {
    fn dyn_clone(&self) -> Box<dyn IPlugin> {
        Box::new(self.clone())
    }
    fn emit_node(&self, node: &KdlNode, context: PluginContext) -> EmitResult<bool> {
        let name = node.name().value();
        let Some(name) = name.strip_prefix('@') else {
            return Ok(false);
        };
        if name == "template" {
            let template_name = node
                .get("name")
                .ok_or_else(|| format!("{name}: Template tags must have a `name` parameter!"))?;
            if node.children().is_none() {
                return Err(format!("{name}: Template tags must have children!"))?;
            }
            self.templates.lock().insert(
                context
                    .emitter
                    .vars
                    .expand_value(template_name)
                    .into_owned(),
                node.clone(),
            );
            Ok(true)
        } else {
            self.emit_template(name, node, context)
        }
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
