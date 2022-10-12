use otto::{crdt::Crdt, list::List, map::Map, mappable_register::MappableRegister as Register, set::Set, State, StateTest, text::Text};
use rand::{Rng, rngs::SmallRng, SeedableRng};

#[ignore]
#[test]
fn fuzz_register() {
	fuzz::<Register<u8>>();
}

#[ignore]
#[test]
fn fuzz_text() {
	fuzz::<Text>();
}

#[ignore]
#[test]
fn fuzz_list() {
	fuzz::<List<u8>>();
}

#[ignore]
#[test]
fn fuzz_tuple() {
	fuzz::<(Text, Text)>();
}

#[ignore]
#[test]
fn fuzz_struct() {
	fuzz::<Register<FooStruct>>();
}

#[ignore]
#[test]
fn fuzz_enum() {
	fuzz::<Register<FooEnum>>();
}

#[ignore]
#[test]
fn fuzz_set() {
	fuzz::<Set<u8>>();
}

#[ignore]
#[test]
fn fuzz_map() {
	fuzz::<Map<u8, Text>>();
}

#[test]
fn fuzz_short_register() {
	fuzz_short::<Register<u8>>();
}

#[test]
fn fuzz_short_text() {
	fuzz_short::<Text>();
}

#[test]
fn fuzz_short_list() {
	fuzz_short::<List<u8>>();
}

#[test]
fn fuzz_short_tuple() {
	fuzz_short::<(Text, Text)>();
}

#[test]
fn fuzz_short_struct() {
	fuzz_short::<Register<FooStruct>>();
}

#[test]
fn fuzz_short_enum() {
	fuzz_short::<Register<FooEnum>>();
}

#[test]
fn fuzz_short_set() {
	fuzz_short::<Set<u8>>();
}

#[test]
fn fuzz_short_map() {
	fuzz_short::<Map<u8, Text>>();
}

#[derive(Clone, PartialEq, Eq, State, StateTest, Debug)]
struct FooStruct {
	a: Text,
	b: u8,
	c: Register<u8>,
	d: Register<Text>,
	e: List<(Register<u8>, Text)>,
}

#[derive(Clone, PartialEq, Eq, State, StateTest, Debug)]
enum FooEnum {
	A(Text),
	B(u8),
	C(Register<u8>),
	D(Register<Text>),
	E(List<(Register<u8>, Text)>),
}

fn fuzz<T: StateTest>() {
	let seed = rand::random();
	println!("seed: {seed}");
	let rng = &mut SmallRng::seed_from_u64(seed);
	for i in 0..u64::MAX {
		if i % 100_000 == 0 {
			println!("{}", i);
		}
		fuzz_once::<T>(rng);
	}
}

fn fuzz_short<T: StateTest>() {
	let seed = rand::random();
	println!("seed: {seed}");
	let rng = &mut SmallRng::seed_from_u64(seed);
	for _ in 0..100 {
		fuzz_once::<T>(rng);
	}
}

fn fuzz_once<T: StateTest>(rng: &mut impl Rng) {
	let mut a = Crdt::new(T::gen(rng));
	let mut b = a.clone();

	println!("start: {:?}", *a);

	for _ in 0..rng.gen_range(1..5) {
		let instr = if a.instrs().len() == 0 || rng.gen_range(0..5) != 0 {
			let instr = StateTest::gen_trivial_instr(&*a, rng).unwrap();
			Crdt::instr_to_crdt_instr(&a, instr)
		} else {
			let mut undos = a.instrs();
			let undo = rng.gen_range(0..undos.len());
			undos.nth(undo).unwrap().inverse()
		};
		a.apply(instr);
	}

	for _ in 0..rng.gen_range(1..5) {
		let instr = if b.instrs().len() == 0 || rng.gen_range(0..5) != 0 {
			let instr = StateTest::gen_trivial_instr(&*b, rng).unwrap();
			Crdt::instr_to_crdt_instr(&b, instr)
		} else {
			let mut undos = b.instrs();
			let undo = rng.gen_range(0..undos.len());
			undos.nth(undo).unwrap().inverse()
		};
		b.apply(instr);
	}

	let a_instrs: Vec<_> = a.instrs().collect();
	let b_instrs: Vec<_> = b.instrs().collect();

	println!("a: {:?}\nb: {:?}", a, b);

	b.apply_multiple(a_instrs);
	a.apply_multiple(b_instrs);

	println!("end: {:?}\n", *a);

	assert_eq!(a, b);
}
