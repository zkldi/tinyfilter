#![forbid(unsafe_code)]

use js_sys::{Array, Error as JsError, Object, Reflect};
use tinyfilter::{Context, Error, Value};
use wasm_bindgen::{JsCast, prelude::*};

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT_TYPES: &str = r#"
export type ExprValue = null | boolean | number | string | {
    [key: string]: ExprValue;
};

export type ExprContext = Record<string, ExprValue>;

export interface TinyfilterError extends Error {
    name: "TinyfilterError";
    phase: "context" | "evaluate";
    kind: "parse" | "unknownVariable" | "unknownFunction" | "type" | "arithmetic" | "limit" | "invalidContext" | "bridge";
    offset?: number;
    limit?: number;
    limitKind?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ExprContext")]
    pub type ExprContext;

    #[wasm_bindgen(typescript_type = "ExprValue")]
    pub type ExprValue;
}

#[wasm_bindgen]
pub fn evaluate(source: &str, context: &ExprContext) -> Result<bool, JsValue> {
    let context = Converter.context(context.as_ref())?;
    tinyfilter::evaluate(source, &context).map_err(|error| core_error(error, "evaluate"))
}

struct Converter;

impl Converter {
    fn context(&self, value: &JsValue) -> Result<Context, JsValue> {
        if !value.is_object() || value.is_null() || Array::is_array(value) {
            return Err(context_error("context must be an object"));
        }
        Ok(Context::from_values(self.object(value)?))
    }

    fn value(&self, value: JsValue) -> Result<Value, JsValue> {
        if value.is_null() {
            return Ok(Value::nil());
        }
        if let Some(value) = value.as_bool() {
            return Ok(Value::bool(value));
        }
        if let Some(value) = value.as_string() {
            return Ok(Value::string(value));
        }
        if value.is_bigint() {
            return Err(context_error("bigint is not an expression value"));
        }
        if let Some(number) = value.as_f64() {
            return Value::num(number).map_err(|error| core_error(error, "context"));
        }
        if value.is_undefined() {
            return Err(context_error("undefined is not an expression value"));
        }
        if Array::is_array(&value) {
            return Err(context_error("arrays are not expression values"));
        }
        if !value.is_object() {
            return Err(context_error(
                "expression values must be null, booleans, numbers, strings, or objects",
            ));
        }

        Ok(Value::map(self.object(&value)?))
    }

    fn object(&self, value: &JsValue) -> Result<Vec<(String, Value)>, JsValue> {
        let entries = Object::entries(value.unchecked_ref());
        let mut values = Vec::with_capacity(entries.length() as usize);
        for entry in entries.iter() {
            let entry = Array::from(&entry);
            let name = entry
                .get(0)
                .as_string()
                .ok_or_else(|| context_error("map keys must be strings"))?;
            values.push((name, self.value(entry.get(1))?));
        }
        Ok(values)
    }
}

fn context_error(message: &str) -> JsValue {
    bridge_error(JsValue::from_str(message), "context", "invalidContext")
}

fn bridge_error(error: JsValue, phase: &str, kind: &str) -> JsValue {
    let message = error
        .as_string()
        .unwrap_or_else(|| "JavaScript bridge error".into());
    structured_error(&message, phase, kind, None, None, None)
}

fn core_error(error: Error, phase: &str) -> JsValue {
    let message = error.to_string();
    let (kind, offset, limit, limit_kind) = match &error {
        Error::Parse { offset, .. } => ("parse", Some(*offset), None, None),
        Error::UnknownVariable(_) => ("unknownVariable", None, None, None),
        Error::UnknownFunction(_) => ("unknownFunction", None, None, None),
        Error::Type(_) => ("type", None, None, None),
        Error::Arithmetic(_) => ("arithmetic", None, None, None),
        Error::Limit { kind, limit } => ("limit", None, Some(*limit), Some(kind.to_string())),
    };
    structured_error(&message, phase, kind, offset, limit, limit_kind.as_deref())
}

fn structured_error(
    message: &str,
    phase: &str,
    kind: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    limit_kind: Option<&str>,
) -> JsValue {
    let error = JsError::new(message);
    error.set_name("TinyfilterError");
    set_property(&error, "phase", &JsValue::from_str(phase));
    set_property(&error, "kind", &JsValue::from_str(kind));
    if let Some(offset) = offset {
        set_property(&error, "offset", &JsValue::from_f64(offset as f64));
    }
    if let Some(limit) = limit {
        set_property(&error, "limit", &JsValue::from_f64(limit as f64));
    }
    if let Some(limit_kind) = limit_kind {
        set_property(&error, "limitKind", &JsValue::from_str(limit_kind));
    }
    error.into()
}

fn set_property(object: &JsError, name: &str, value: &JsValue) {
    let _ = Reflect::set(object.as_ref(), &JsValue::from_str(name), value);
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn evaluates_objects() {
        let context = Object::new();
        Reflect::set(
            context.as_ref(),
            &JsValue::from_str("chart"),
            &Object::from_entries(
                &Array::of2(
                    &Array::of2(&JsValue::from_str("level"), &JsValue::from_f64(12.0)),
                    &Array::of2(&JsValue::from_str("title"), &JsValue::from_str("Blue Rain")),
                )
                .into(),
            )
            .unwrap(),
        )
        .unwrap();

        let context: ExprContext = JsValue::from(context).into();
        assert!(
            evaluate(
                r#"chart.level >= 10 and contains(chart.title, "Rain")"#,
                &context
            )
            .unwrap()
        );
    }

    #[wasm_bindgen_test]
    fn throws_structured_errors() {
        let context: ExprContext = JsValue::from(Object::new()).into();
        let error = evaluate("unknown()", &context).unwrap_err();
        let property = |name| Reflect::get(&error, &JsValue::from_str(name)).unwrap();
        assert_eq!(
            property("name").as_string().as_deref(),
            Some("TinyfilterError")
        );
        assert_eq!(property("phase").as_string().as_deref(), Some("evaluate"));
        assert_eq!(
            property("kind").as_string().as_deref(),
            Some("unknownFunction")
        );
    }

    #[wasm_bindgen_test]
    fn requires_a_boolean_result() {
        let context: ExprContext = JsValue::from(Object::new()).into();
        assert!(!evaluate("false", &context).unwrap());
        assert!(evaluate("true", &context).unwrap());
        assert!(evaluate("nil", &context).is_err());
        assert!(evaluate("42", &context).is_err());
    }
}
