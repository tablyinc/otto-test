use otto::set::Set;

use common::{fuzz, fuzz_short};

mod common;

#[ignore]
#[test]
fn fuzz_set() {
	fuzz::<Set<u8>>();
}

#[test]
fn fuzz_short_set() {
	fuzz_short::<Set<u8>>();
}
