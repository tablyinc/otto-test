use otto::{crdt::Crdt, StateTest};
use rand::{Rng, rngs::SmallRng, SeedableRng};

pub(super) fn fuzz<T: StateTest>() {
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

pub(super) fn fuzz_short<T: StateTest>() {
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

	let a_instrs = a.instrs().collect::<Vec<_>>();
	let b_instrs = b.instrs().collect::<Vec<_>>();

	println!("a: {:?}\nb: {:?}", a, b);

	b.apply_multiple(a_instrs);
	a.apply_multiple(b_instrs);

	println!("end: {:?}\n", *a);

	assert_eq!(a, b);
}
