use htmeta::{kdl::KdlValue, ScriptingError, Value, Vars};
use rhai::{Array, Dynamic};

pub(crate) fn parse_range(vars: &Vars, args: &[&KdlValue]) -> Result<Array, ScriptingError> {
    let command = &args[0];
    let args = args.into_iter().copied().map(Value::from).map(Value::into_dynamic);
    if let Some(command) = command.as_string().and_then(|i| i.strip_prefix("@")) {
        let iter = vars.call_func::<Dynamic>(command, args.skip(1).collect())?;
        Ok(vars.call_func("array", vec![iter])?)
    } else {
        Ok(args.collect())
    }
}
