//! adapted from https://github.com/josephg/diamond-types
//!
//! ISC License
//!
//! Copyright 2022 the Diamond Types contributors
//!
//! Permission to use, copy, modify, and/or distribute this software for any
//! purpose with or without fee is hereby granted, provided that the above
//! copyright notice and this permission notice appear in all copies.
//!
//! THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
//! WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
//! MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
//! SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//! WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION
//! OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN
//! CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

#![feature(generic_arg_infer)]

use std::hash::Hash;

use diamond_types::list::{fuzzer_tools, fuzzer_tools::make_random_change, ListCRDT};
use hashbag::HashBag;
use index_many::generic::{get_many_mut, UnsortedIndices};
use otto::{crdt::Crdt, list::List, State, StateTest};
use rand::prelude::*;

use helpers::{doc_to_string, replicate_random_change};

mod helpers;

fn make_random_change_fuzz<const VERBOSE: bool>(seed: u64) {
	let mut rng = SmallRng::seed_from_u64(seed);
	let mut diamond = ListCRDT::new();
	diamond.get_or_create_agent_id("agent 0");
	let mut otto = Crdt::new(List::new());

	for i in 0..200 {
		if VERBOSE {
			println!("\n\ni {i}");
		}

		let prev_oplog = diamond.oplog.clone();
		make_random_change(&mut diamond, None, 0 as _, &mut rng);
		replicate_random_change(&mut otto, &prev_oplog, &diamond.oplog);
		assert_eq!(diamond.branch.content.to_string(), doc_to_string(&otto));
	}
}

#[test]
fn make_random_change_fuzz_once() {
	make_random_change_fuzz::<true>(321);
}

#[test]
#[ignore]
fn make_random_change_fuzz_forever() {
	for seed in 0.. {
		if seed % 10 == 0 {
			println!("seed {seed}");
		}
		make_random_change_fuzz::<false>(seed);
	}
}

pub fn add_missing_operations_from<T>(to: &mut Crdt<T>, from: &Crdt<T>)
where
	T: State + Eq + Hash,
	T::Instr: Eq + Hash,
{
	let self_: HashBag<_> = to.instrs().collect();
	let from_: HashBag<_> = from.instrs().collect();
	for (instr, count) in from_.difference(&self_) {
		for _ in 0..count {
			to.apply(instr.clone());
		}
	}
}

#[test]
fn add_missing_operations_from_converges() {
	let rng = &mut SmallRng::seed_from_u64(42);
	for _ in 0..100 {
		let state = <List<u64>>::gen(rng);
		let mut crdt_a = Crdt::gen_from_state(rng, &state, 10);
		let mut crdt_b = Crdt::gen_from_state(rng, &state, 10);
		add_missing_operations_from(&mut crdt_a, &crdt_b);
		add_missing_operations_from(&mut crdt_b, &crdt_a);
		assert!(crdt_a.converges(&crdt_b), "{:?}\n{:?}", crdt_a, crdt_b);
	}
}

fn oplog_merge_fuzz<const VERBOSE: bool>(seed: u64) {
	let mut rng = SmallRng::seed_from_u64(seed);
	let mut diamonds = [ListCRDT::new(), ListCRDT::new(), ListCRDT::new()];
	let mut ottos = [Crdt::new(List::new()), Crdt::new(List::new()), Crdt::new(List::new())];

	for i in 0..diamonds.len() {
		for a in 0..diamonds.len() {
			diamonds[i].get_or_create_agent_id(format!("agent {a}").as_str());
		}
	}

	for i in 0..200 {
		if VERBOSE {
			println!("\n\ni {i}");
		}

		for _ in 0..2 {
			let idx = rng.gen_range(0..diamonds.len());
			let prev_oplog = diamonds[idx].oplog.clone();
			make_random_change(&mut diamonds[idx], None, idx as _, &mut rng);
			replicate_random_change(&mut ottos[idx], &prev_oplog, &diamonds[idx].oplog);
			debug_assert_eq!(diamonds[idx].branch.content.to_string(), doc_to_string(&ottos[idx]));
		}

		let (idx_a, a_diamond, idx_b, b_diamond) = fuzzer_tools::choose_2(&mut diamonds, &mut rng);
		let [a_otto, b_otto] = get_many_mut(&mut ottos, UnsortedIndices([idx_a, idx_b])).unwrap();

		a_diamond.oplog.add_missing_operations_from(&b_diamond.oplog);
		b_diamond.oplog.add_missing_operations_from(&a_diamond.oplog);
		debug_assert_eq!(a_diamond.oplog, b_diamond.oplog);

		a_diamond.branch.merge(&a_diamond.oplog, &a_diamond.oplog.version);
		b_diamond.branch.merge(&b_diamond.oplog, &b_diamond.oplog.version);
		debug_assert_eq!(a_diamond.branch.content, b_diamond.branch.content);

		add_missing_operations_from(a_otto, b_otto);
		add_missing_operations_from(b_otto, a_otto);
		debug_assert_eq!(doc_to_string(&a_otto), doc_to_string(&b_otto));

		assert_eq!(a_diamond.branch.content.to_string(), doc_to_string(&a_otto));
		assert_eq!(b_diamond.branch.content.to_string(), doc_to_string(&b_otto));
	}
}

#[test]
#[ignore] // TODO investigate why the document state we converge to is different from Seph's
fn oplog_merge_fuzz_once() {
	oplog_merge_fuzz::<true>(321);
}

#[test]
#[ignore]
fn oplog_merge_fuzz_forever() {
	for seed in 0.. {
		if seed % 10 == 0 {
			println!("seed {seed}");
		}
		oplog_merge_fuzz::<false>(seed);
	}
}
