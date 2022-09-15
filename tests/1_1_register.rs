use otto::settable::Settable as Register;

use common::{fuzz, fuzz_short};

mod common;

#[ignore]
#[test]
fn fuzz_list() {
	fuzz::<Register<u8>>();
}

#[test]
fn fuzz_short_list() {
	fuzz_short::<Register<u8>>();
}
