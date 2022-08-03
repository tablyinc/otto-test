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

use std::ops::Range;

use all_asserts::{assert_gt, debug_assert_gt};
use diamond_types::list::{
    fuzzer_tools,
    fuzzer_tools::make_random_change,
    operation::{OpKind, Operation},
    ListCRDT,
};
use otto::{
    crdt::Crdt,
    list::{List, ListInstr},
    State,
};
use rand::prelude::*;

fn last_n_ops(crdt: &ListCRDT, n: usize) -> impl Iterator<Item = Operation> + '_ {
    crdt.oplog.operations.0[crdt.oplog.operations.0.len() - n..]
        .iter()
        .map(|op| op.1.to_operation(&crdt.oplog))
}

fn get_char_range(op: &Operation) -> Range<usize> {
    if op.loc.fwd {
        Range {
            start: op.loc.span.start,
            end: op.loc.span.end,
        }
    } else {
        Range {
            start: op.loc.span.end,
            end: op.loc.span.start,
        }
    }
}

fn to_utf8_range(doc: &List<u8>, char_range: &Range<usize>) -> Range<usize> {
    let string = doc_to_string(&doc);
    let offset = string
        .chars()
        .take(char_range.start)
        .collect::<String>()
        .len();
    let span = string
        .chars()
        .skip(char_range.start)
        .take(char_range.end - char_range.start)
        .collect::<String>()
        .len();
    offset..offset + span
}

fn convert(crdt: &Crdt<List<u8>>, op: &Operation) -> Vec<ListInstr<u8>> {
    debug_assert!(op.content.is_some());
    let mut ops = vec![];
    let mut doc = (**crdt).clone();
    match op.kind {
        OpKind::Ins => {
            debug_assert!(op.loc.fwd);
            for (i, x) in op.content.as_ref().unwrap().as_bytes().iter().enumerate() {
                let ins = doc.insert(op.loc.span.start + i, *x);
                doc.apply(&ins);
                ops.push(ins);
            }
        }
        OpKind::Del => {
            let char_range = get_char_range(&op);
            let utf8_range = to_utf8_range(&doc, &char_range);
            for _ in 0..utf8_range.len() {
                let del = doc.delete(utf8_range.start);
                doc.apply(&del);
                ops.push(del);
            }
        }
    }
    ops
}

fn doc_to_string(doc: &List<u8>) -> String {
    String::from_utf8((0..doc.len()).map(|at| doc[at]).collect::<Vec<_>>()).unwrap()
}

fn make_random_change_fuzz<const VERBOSE: bool>(seed: u64) {
    let mut rng = SmallRng::seed_from_u64(seed);

    let mut diamond = ListCRDT::new();
    diamond.get_or_create_agent_id("agent 0");

    let mut otto = <Crdt<List<u8>>>::new(List::new());
    // how many otto instructions was diamond types' last operation
    let mut last_n = 0;

    for _i in 0..200 {
        if VERBOSE {
            println!("\n\ni {_i}");
        }

        let prev_len = diamond.oplog.operations.0.len();
        make_random_change(&mut diamond, None, 0 as _, &mut rng);
        let curr_len = diamond.oplog.operations.0.len();

        // same type consecutive operations at the end get compressed into an updated last operation
        if curr_len == prev_len {
            // undo diamond types' last operation from previous run
            debug_assert_gt!(last_n, 0);
            let mut undos: Vec<_> = otto.instrs_().rev().take(last_n).rev().cloned().collect();
            <List<_>>::inverse_multiple(&mut undos);
            otto.apply_multiple_(undos);
        }

        // now we are ready to apply new operations - or redo the last operation if it was updated
        for diamond_op in last_n_ops(&diamond, 1.max(curr_len - prev_len)) {
            let instrs = convert(&mut otto, &diamond_op);
            last_n = instrs.len();
            for instr in instrs {
                otto.apply_(instr);
            }
        }

        // TODO investigate why this fails (off-by-one bug in my operation/instructions conversion?)
        dbg!(diamond.branch.content.to_string());
        dbg!(doc_to_string(&otto));
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

// TODO make this into a fuzzing test that runs otto alongside diamond types and compares them
fn oplog_merge_fuzz<const VERBOSE: bool>(seed: u64) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut docs = [ListCRDT::new(), ListCRDT::new(), ListCRDT::new()];

    for i in 0..docs.len() {
        // docs[i].get_or_create_agent_id(format!("agent {}", i).as_str());
        for a in 0..docs.len() {
            docs[i].get_or_create_agent_id(format!("agent {}", a).as_str());
        }
    }

    for _i in 0..200 {
        if VERBOSE {
            println!("\n\ni {}", _i);
        }

        // for (idx, d) in docs.iter().enumerate() {
        //     println!("doc {idx} length {}", d.ops.len());
        // }

        // Generate some operations
        for _j in 0..2 {
            let idx = rng.gen_range(0..docs.len());

            // This should + does also work if we set idx=0 and use the same agent for all changes.
            // make_random_change(&mut docs[idx], None, 0, &mut rng);
            make_random_change(&mut docs[idx], None, idx as _, &mut rng);
        }

        // for (idx, d) in docs.iter().enumerate() {
        //     println!("with changes {idx} length {}", d.ops.len());
        // }

        let (_a_idx, a, _b_idx, b) = fuzzer_tools::choose_2(&mut docs, &mut rng);

        // a.ops.dbg_print_assignments_and_ops();
        // println!("\n");
        // b.ops.dbg_print_assignments_and_ops();

        // dbg!((&a.ops, &b.ops));
        a.oplog.add_missing_operations_from(&b.oplog);
        // a.check(true);
        // println!("->c {_a_idx} length {}", a.ops.len());

        b.oplog.add_missing_operations_from(&a.oplog);
        // b.check(true);
        // println!("->c {_b_idx} length {}", b.ops.len());

        // dbg!((&a.ops, &b.ops));

        assert_eq!(a.oplog, b.oplog);

        a.branch.merge(&a.oplog, &a.oplog.version);
        b.branch.merge(&b.oplog, &b.oplog.version);
        assert_eq!(a.branch.content, b.branch.content);
    }

    for doc in &docs {
        doc.dbg_check(true);
    }
}

#[test]
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
