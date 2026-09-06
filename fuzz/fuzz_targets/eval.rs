#![no_main]

use libfuzzer_sys::fuzz_target;
use tinyfilter::{Context, Value};

fuzz_target!(|data: &[u8]| {
	let expressions = [
		"left + right",
		"left - right",
		"left * right",
		"left / right",
		"left % right",
		"left ** right",
		"left < right and right <= 10",
		"num(input) >= 0 and num(input) <= 10",
		"startsWith(input, \"12\")",
		"min(left, right)",
		"contains(input, \"x\")",
	];
	let arbitrary = String::from_utf8_lossy(data);
	let source = data
		.first()
		.filter(|selector| **selector & 1 == 1)
		.map(|selector| expressions[usize::from(*selector) % expressions.len()])
		.unwrap_or(&arbitrary);
	let mut context = Context::default();
	let value = String::from_utf8_lossy(data.get(..data.len().min(4096)).unwrap_or(data));
	context.insert("input", Value::string(value.as_ref()));
	let left = data
		.get(..8)
		.and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
		.map(i64::from_le_bytes)
		.unwrap_or_default();
	let right = data
		.get(8..16)
		.and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
		.map(i64::from_le_bytes)
		.unwrap_or_default();
	context.insert("left", Value::num(left as f64).unwrap());
	context.insert("right", Value::num(right as f64).unwrap());
	let _ = tinyfilter::evaluate(&source, &context);
});
