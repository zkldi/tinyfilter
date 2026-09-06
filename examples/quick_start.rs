use tinyfilter::{Context, Value, evaluate};

fn main() -> tinyfilter::Result<()> {
    let mut chart = Context::default();
    chart.insert("level", Value::string("12"));
    chart.insert("desc", Value::string("EXPERT"));
    chart.insert(
        "tags",
        Value::map([
            ("kind".into(), Value::string("boss")),
            ("author".into(), Value::string("Alice")),
        ]),
    );

    let matches = evaluate(
        r#"
			num(level) >= 12
				and tags.kind == "boss"
				and exists tags.author
		"#,
        &chart,
    )?;

    println!("chart matches: {matches}");
    Ok(())
}
