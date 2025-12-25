use htmeta::kdl::KdlValue;

pub(crate) fn parse_range(args: &[&KdlValue]) -> Option<impl Iterator<Item=i64>> {
    let maybe_command = args.first().and_then(|it| it.as_string())?;
    if maybe_command != "@range" {
        return None;
    }
    let args = &args[1..];

    let args = args
        .iter()
        .map(|val| val.as_integer().map(|i| i as i64))
        .collect::<Option<Vec<i64>>>()?;

    let mut start = 1;
    let mut step = 1;
    let end = match args.len() {
        1 => args[0],
        2 => {
            start = args[0];
            args[1]
        }
        3 => {
            start = args[0];
            step = args[1]as usize;
            args[2]
        }
        _ => return None,
    };

    Some((start..=end).step_by(step))
}
