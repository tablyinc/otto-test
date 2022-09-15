use otto::{map::Map, text::Text};

use common::{fuzz, fuzz_short};

mod common;

#[ignore]
#[test]
fn fuzz_map() {
	fuzz::<Map<u8, Text>>();
}

#[test]
fn fuzz_short_map() {
	fuzz_short::<Map<u8, Text>>();
}
