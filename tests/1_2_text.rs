use otto::text::Text;

use common::{fuzz, fuzz_short};

mod common;

#[ignore]
#[test]
fn fuzz_list() {
	fuzz::<Text>();
}

#[test]
fn fuzz_short_list() {
	fuzz_short::<Text>();
}
