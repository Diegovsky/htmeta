use std::any::Any;

use super::*;

/// Information that plugins can use to change what is being emitted.
///
/// Check out [`HtmlEmitter`] for more information!
pub struct PluginContext<'a, 'b, 'c, Emitter> {
    /// Pre-computed indentation from the current level.
    pub indent: &'a str,
    /// The [`Writer`] handle we're currently emitting into.
    pub writer: &'b mut Writer<'c>,
    /// A handle to the current node's emitter.
    pub emitter: Emitter,
}

pub enum EmitStatus {
    Skip,
    Emit,
    EmitMut,
}

/// A trait that allows you to hook into `htmeta`'s emitter and extend it!
pub trait IPlugin: DynClone + Any {
    fn should_emit(&self, node: &KdlNode, emitter: &HtmlEmitter) -> EmitStatus;
    fn emit_node(&self, node: &KdlNode, context: PluginContext<&HtmlEmitter>) -> EmitResult;
    fn emit_node_mut(
        &mut self,
        node: &KdlNode,
        context: PluginContext<&mut HtmlEmitter>,
    ) -> EmitResult;
}

#[derive(Clone)]
pub struct Plugin(Rc<dyn IPlugin>);

impl Plugin {
    pub fn new<P: IPlugin + 'static>(plugin: P) -> Self {
        Self(Rc::new(plugin))
    }

    pub fn make_mut(&mut self) -> &mut dyn IPlugin {
        dyn_clone::rc_make_mut(&mut self.0)
    }

    pub fn get_plugin<T: IPlugin>(&self) -> Option<&T> {
        let it = &*self.0 as &dyn Any;
        it.downcast_ref::<T>()
    }

}

impl std::ops::Deref for Plugin {
    type Target = dyn IPlugin;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
