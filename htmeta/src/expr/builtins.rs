use rhai::{Array, Dynamic, Engine, Module, Scope, AST, INT};

fn lorem(count: INT) -> Array {
    include_str!("lorem.txt")
        .split_whitespace()
        .cycle()
        .take(count.abs() as usize)
        .map(Dynamic::from)
        .collect()
}

pub fn make_engine() -> Engine {
    let mut engine = Engine::new();
    let mod_src =include_str!("builtins.rhai");
    let ast = engine.compile(mod_src).unwrap();
    let module = Module::eval_ast_as_new(Scope::new(), &ast, &engine).unwrap();
    engine.register_global_module(module.into());
    engine.register_fn("lorem", lorem);
    engine
}
