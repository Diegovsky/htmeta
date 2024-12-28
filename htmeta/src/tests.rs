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

auto_html_test_fail!(fail_mixed_text);

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
