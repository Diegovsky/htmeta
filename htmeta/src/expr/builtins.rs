use rhai::{Engine, Module, Scope, INT};

fn lorem(count: INT) -> String {
    let count = count.abs() as usize;
    let mut nexti = 80;
    include_str!("lorem.txt")
        .split_whitespace()
        .cycle()
        .enumerate()
        .take(count)
                // ensure line breaks happen after sentence ends and it ends with a dot
        .flat_map(|(i, w)| if w.ends_with(".") && (i >= nexti || i == count-1 ){
            nexti += 40;
            vec![w, "\n<br>\n"]
        } else if dbg!(i) == count-1 {vec!["endus."]} else {vec![w]})
        .collect::<Vec<_>>()
        .join(" ")
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
