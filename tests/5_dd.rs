#![allow(clippy::if_not_else, clippy::range_plus_one)]

use rand::{rngs::SmallRng, Rng, SeedableRng};

use otto::{crdt::Crdt, text::Text, StateTest};

fn test_crdt_differential_dataflow<T: StateTest>(rng: &mut impl Rng) {
    let mut upstream_crdt = Crdt::new(T::gen(rng));
    for _ in 0..rng.gen_range(0..100) {
        let upstream_instr = StateTest::gen_trivial_instr(&*upstream_crdt, rng).unwrap();
        upstream_crdt.apply_(upstream_instr);
    }

    let mut downstream_crdt = upstream_crdt.clone();
    for _ in 0..rng.gen_range(0..100) {
        let downstream_instr = StateTest::gen_trivial_instr(&*downstream_crdt, rng).unwrap();
        downstream_crdt.apply_(downstream_instr);
    }

    for _ in 0..1 {
        let upstream_instr = StateTest::gen_trivial_instr(&*upstream_crdt, rng).unwrap();
        let crdt_instr = upstream_crdt.instr_to_crdt_instr(upstream_instr.clone());
        print!("upstream: {:?} -> ", *upstream_crdt);
        upstream_crdt.apply_(upstream_instr.clone());
        println!("{:?}", *upstream_crdt);

        let downstream_instr = downstream_crdt.instr_from_crdt_instr_(crdt_instr);
        print!("downstream: {:?} -> ", *downstream_crdt);
        downstream_crdt.apply_(downstream_instr.clone());
        println!("{:?}\n", *downstream_crdt);
    }
}

#[ignore]
#[test]
fn fuzz_crdt_differential_dataflow() {
    let seed = rand::random();
    println!("seed: {seed}");
    let rng = &mut SmallRng::seed_from_u64(seed);
    for i in 0..u64::MAX {
        if i % 1_000 == 0 {
            println!("{}", i);
        }
        test_crdt_differential_dataflow::<Text>(rng);
    }
}

#[test]
fn fuzz_crdt_differential_dataflow_short() {
    let seed = rand::random();
    println!("seed: {seed}");
    let rng = &mut SmallRng::seed_from_u64(seed);
    for _ in 0..100 {
        test_crdt_differential_dataflow::<Text>(rng);
    }
}
