use otto::list::List;

use common::{fuzz, fuzz_short};

mod common;

#[ignore]
#[test]
fn fuzz_list() {
	fuzz::<List<u8>>();
}

#[test]
fn fuzz_short_list() {
	fuzz_short::<List<u8>>();
}
