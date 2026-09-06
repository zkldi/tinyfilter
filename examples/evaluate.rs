use std::{collections::BTreeMap, env, process};

use tinyfilter::{Context, Value, evaluate};

fn usage() -> ! {
    eprintln!("usage: evaluate '<expression>' [name=value ...]");
    process::exit(2);
}

fn parse_value(value: &str) -> Value {
    match value {
        "true" => Value::bool(true),
        "false" => Value::bool(false),
        "nil" => Value::nil(),
        value => value
            .parse::<f64>()
            .ok()
            .and_then(|value| Value::num(value).ok())
            .unwrap_or_else(|| Value::string(value)),
    }
}

fn insert_path(values: &mut BTreeMap<String, Value>, path: &[&str], value: Value) {
    if let [key] = path {
        values.insert((*key).into(), value);
        return;
    }

    let mut nested = values
        .remove(path[0])
        .and_then(|value| value.as_map().cloned())
        .unwrap_or_default();
    insert_path(&mut nested, &path[1..], value);
    values.insert(path[0].into(), Value::map(nested));
}

fn main() -> tinyfilter::Result<()> {
    let mut args = env::args().skip(1);
    let source = args.next().unwrap_or_else(|| usage());

    let mut values = BTreeMap::new();
    for assignment in args {
        let (path, value) = assignment.split_once('=').unwrap_or_else(|| usage());
        let path = path.split('.').collect::<Vec<_>>();
        if path.iter().any(|part| part.is_empty()) {
            usage();
        }
        insert_path(&mut values, &path, parse_value(value));
    }

    let matches = evaluate(&source, &Context::from_values(values))?;
    println!("{matches}");
    Ok(())
}
