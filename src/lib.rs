#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

mod error;
mod eval;
mod lexer;
mod parser;
mod value;

pub use error::{Error, LimitKind, Result};
use parser::compile;
pub use value::{Context, Value};

pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
pub const MAX_NODES: usize = 8_192;
pub const MAX_DEPTH: usize = 64;

/// Evaluate a user-provided filter on your provided [`Context`].
pub fn evaluate(source: &str, context: &Context) -> Result<bool> {
    let program = compile(source)?;
    program
        .eval(context)?
        .as_bool()
        .ok_or(Error::Type("filter must evaluate to a boolean"))
}

#[cfg(test)]
fn evaluate_value(source: &str, context: &Context) -> Result<Value> {
    let program = compile(source)?;
    Ok(program.eval(context)?.into_value())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn scalar_arithmetic_and_comparisons_are_typed() {
        let context = Context::default();
        assert_eq!(
            evaluate_value("5 / 2", &context).unwrap().as_num(),
            Some(2.5)
        );
        assert_eq!(
            evaluate_value("2 + 3 * 4 == 14", &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            evaluate_value("1 < 1.5 and 2.0 >= 2", &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            evaluate_value("not false and true", &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert!(evaluate_value("1 / 0", &context).is_err());
        assert!(evaluate("5.5 % 2 == 1.5", &context).unwrap());
    }

    #[test]
    fn numeric_separators_must_be_between_digits() {
        let context = Context::default();
        assert!(evaluate("1_000 == 1000", &context).is_ok());
        assert!(evaluate("1__1 == 11", &context).is_err());
        assert!(evaluate("1_ == 1", &context).is_err());
        assert!(evaluate("1._5 == 1.5", &context).is_err());
        assert!(evaluate("1.5_ == 1.5", &context).is_err());
    }

    #[test]
    fn contexts_can_be_built_from_values() {
        let context = Context::from_values([("answer".into(), Value::num(42.0).unwrap())]);
        assert_eq!(context.get("answer").unwrap().as_num(), Some(42.0));
    }

    #[test]
    fn primitive_values_convert_into_values() {
        let mut context = Context::default();
        context.insert("integer", 42_i64);
        context.insert("float", 4.5_f64);
        context.insert("borrowed", "text");
        context.insert("owned", String::from("text"));

        assert_eq!(context.get("integer").unwrap().as_num(), Some(42.0));
        assert_eq!(context.get("float").unwrap().as_num(), Some(4.5));
        assert_eq!(context.get("borrowed").unwrap().as_str(), Some("text"));
        assert_eq!(context.get("owned").unwrap().as_str(), Some("text"));
    }

    #[test]
    fn public_api_returns_only_booleans() {
        let context = Context::default();
        assert!(evaluate("true", &context).unwrap());
        assert!(!evaluate("false", &context).unwrap());
        assert!(matches!(evaluate("nil", &context), Err(Error::Type(_))));
        assert!(matches!(evaluate("42", &context), Err(Error::Type(_))));
    }

    #[test]
    fn conversions_make_string_levels_numeric() {
        let mut context = Context::default();
        context.insert("level", Value::string("10"));
        assert_eq!(
            evaluate_value("num(level) >= 10", &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
        context.insert("rating", Value::string("7.5"));
        assert_eq!(
            evaluate_value("num(rating) >= 0 and num(rating) <= 10", &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
    }

    #[test]
    fn map_access_and_string_predicates_work() {
        let mut context = Context::default();
        context.insert(
            "tags",
            Value::map([("kind".into(), Value::string("boss-12"))]),
        );
        assert_eq!(
            evaluate_value(r#"startsWith(tags.kind, "boss")"#, &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            evaluate_value(r#"contains(tags.kind, "boss")"#, &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            evaluate_value(r#"contains(tags.kind, "missing")"#, &context)
                .unwrap()
                .as_bool(),
            Some(false)
        );
    }

    #[test]
    fn maps_cannot_be_compared_for_equality() {
        let mut context = Context::default();
        context.insert("tags", Value::map([]));

        assert!(matches!(
            evaluate("tags == tags", &context),
            Err(Error::Type("maps are not comparable"))
        ));
        assert!(matches!(
            evaluate("tags != 1", &context),
            Err(Error::Type("maps are not comparable"))
        ));
    }

    #[test]
    fn string_literals_can_span_lines() {
        assert!(
            evaluate(
                "\"first\nsecond\" == \"first\\nsecond\"",
                &Context::default()
            )
            .unwrap()
        );
    }

    #[test]
    fn conditionals_indexing_and_otherwise_work() {
        let mut context = Context::default();
        context.insert(
            "tags",
            Value::map([("marathon".into(), Value::bool(false))]),
        );
        context.insert("level", Value::num(12.0).unwrap());
        assert_eq!(
            evaluate_value("if 7 > 5 { 7 } else { 0 }", &context)
                .unwrap()
                .as_num(),
            Some(7.0)
        );
        assert_eq!(
            evaluate_value(r#"tags.missing otherwise "fallback""#, &context)
                .unwrap()
                .as_str(),
            Some("fallback")
        );
        assert_eq!(
            evaluate_value("tags.marathon otherwise unknown", &context)
                .unwrap()
                .as_bool(),
            Some(false)
        );
        assert_eq!(
            evaluate_value("if exists tags.marathon { true } else { false }", &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            evaluate_value(
                "if not exists tags.missing { true } else { false }",
                &context
            )
            .unwrap()
            .as_bool(),
            Some(true)
        );
        assert_eq!(
            evaluate_value("level == 12 and exists tags.marathon", &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            evaluate_value("exists nil", &context).unwrap().as_bool(),
            Some(false)
        );
    }

    #[test]
    fn numeric_and_string_functions_work() {
        let context = Context::default();
        let type_name = evaluate_value("type(42)", &context).unwrap();
        assert_eq!(type_name.as_str(), Some("num"));
        assert_eq!(
            evaluate_value("max(1, 7.5)", &context).unwrap().as_num(),
            Some(7.5)
        );
        assert_eq!(
            evaluate_value(r#"endsWith("level+", "+")"#, &context)
                .unwrap()
                .as_bool(),
            Some(true)
        );
    }

    #[test]
    fn bad_arity_is_an_error_not_a_panic() {
        let context = Context::default();
        assert!(evaluate_value("min(1)", &context).is_err());
    }

    #[test]
    fn every_function_name_resolves() {
        let context = Context::default();
        for function in parser::Function::ALL {
            let name = function.name();
            assert!(
                !matches!(
                    evaluate_value(&format!("{name}()"), &context),
                    Err(Error::UnknownFunction(_))
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn unknown_functions_are_compile_errors() {
        assert!(matches!(
            compile("doesNotExist(1)"),
            Err(Error::UnknownFunction(_))
        ));
    }

    proptest! {
        #[test]
        fn arbitrary_utf8_source_never_panics(source in any::<String>()) {
            let _ = compile(&source);
        }

        #[test]
        fn numeric_arithmetic_never_panics(left in any::<i64>(), right in any::<i64>()) {
            let mut context = Context::default();
            context.insert("left", Value::num(left as f64).unwrap());
            context.insert("right", Value::num(right as f64).unwrap());
            for operator in ["+", "-", "*", "/", "%", "**"] {
                let _ = evaluate_value(&format!("left {operator} right"), &context);
            }
        }

        #[test]
        fn generated_numeric_expressions_never_panic(
            left in any::<i64>(),
            right in any::<i64>(),
            operator in prop::sample::select(vec!["+", "-", "*", "/", "%", "**", "==", "<", ">"]),
        ) {
            let mut context = Context::default();
            context.insert("left", Value::num(left as f64).unwrap());
            context.insert("right", Value::num(right as f64).unwrap());
            let _ = evaluate_value(&format!("left {operator} right"), &context);
        }
    }
}
