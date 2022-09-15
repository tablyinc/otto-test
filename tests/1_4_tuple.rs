use otto::text::Text;

use common::{fuzz, fuzz_short};

mod common;

#[ignore]
#[test]
fn fuzz_tuple() {
	fuzz::<(Text, Text)>();
}

#[test]
fn fuzz_short_tuple() {
	fuzz_short::<(Text, Text)>();
}
