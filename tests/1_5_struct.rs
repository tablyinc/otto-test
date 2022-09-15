use otto::{list::List, settable::Settable as Register, State, StateTest, text::Text};

use common::{fuzz, fuzz_short};

mod common;

#[derive(Clone, PartialEq, Eq, State, StateTest, Debug)]
struct FooStruct {
	a: Text,
	b: u8,
	c: Register<u8>,
	d: Register<Text>,
	e: List<(Register<u8>, Text)>,
}

#[ignore]
#[test]
fn fuzz_struct() {
	fuzz::<Register<FooStruct>>();
}

#[test]
fn fuzz_short_struct() {
	fuzz_short::<Register<FooStruct>>();
}
