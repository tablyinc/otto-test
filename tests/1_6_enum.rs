use otto::{list::List, settable::Settable as Register, State, StateTest, text::Text};

use common::{fuzz, fuzz_short};

mod common;

#[derive(Clone, PartialEq, Eq, State, StateTest, Debug)]
enum FooEnum {
	A(Text),
	B(u8),
	C(Register<u8>),
	D(Register<Text>),
	E(List<(Register<u8>, Text)>),
}

#[ignore]
#[test]
fn fuzz_enum() {
	fuzz::<Register<FooEnum>>();
}

#[test]
fn fuzz_short_enum() {
	fuzz_short::<Register<FooEnum>>();
}
