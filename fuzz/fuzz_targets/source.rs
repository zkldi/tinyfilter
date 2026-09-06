#![no_main]

use libfuzzer_sys::fuzz_target;
use tinyfilter::Context;

fuzz_target!(|data: &[u8]| {
	let source = String::from_utf8_lossy(data);
	let _ = tinyfilter::evaluate(&source, &Context::default());
});
