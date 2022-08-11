use std::ops::Range;

use all_asserts::{assert_lt, debug_assert_lt};
use diamond_types::list::{
	operation::{OpKind, Operation}, OpLog
};
use otto::{
	crdt::Crdt, list::{List, ListInstr}, State
};

struct CharRange(pub Range<usize>);

struct Utf8Range(pub Range<usize>);

pub fn doc_to_string(doc: &List<u8>) -> String {
	String::from_utf8((0..doc.len()).map(|at| doc[at]).collect::<Vec<_>>()).unwrap()
}

// outside crate so can't implement as trait
fn not(op: Operation) -> Operation {
	let mut nop = op.clone();
	// we create forwards deletions as inserts don't have a unique inverse
	nop.loc.fwd = true;
	nop.loc.span.start = op.loc.span.start.min(op.loc.span.end);
	nop.loc.span.end = op.loc.span.start.max(op.loc.span.end);
	nop.kind = match op.kind {
		OpKind::Ins => OpKind::Del,
		OpKind::Del => OpKind::Ins,
	};
	nop
}

fn diff_first_idx(self_: &OpLog, other: &OpLog) -> Option<usize> {
	for i in 0..self_.operations.0.len().min(other.operations.0.len()) {
		if self_.operations.0[i] != other.operations.0[i] {
			// neither oplog is a prefix/suffix of the other as they first differ here
			return Some(i);
		}
	}
	None
}

fn last_n_ops(oplog: &OpLog, n: usize) -> impl DoubleEndedIterator<Item = Operation> + '_ {
	oplog.operations.0[oplog.operations.0.len() - n..].iter().map(move |op| op.1.to_operation(&oplog))
}

fn get_char_range(op: &Operation) -> CharRange {
	CharRange(Range { start: op.loc.span.start.min(op.loc.span.end), end: op.loc.span.start.max(op.loc.span.end) })
}

fn to_utf8_range(doc: &List<u8>, char_range: &CharRange) -> Utf8Range {
	let string = doc_to_string(&doc);
	let offset = string.chars().take(char_range.0.start).map(|char| char.len_utf8()).sum();
	let span: usize = string.chars().skip(char_range.0.start).take(char_range.0.end - char_range.0.start).map(|char| char.len_utf8()).sum();
	Utf8Range(offset..offset + span)
}

fn convert(crdt: &Crdt<List<u8>>, op: &Operation) -> Vec<ListInstr<u8>> {
	debug_assert!(op.content.is_some());
	let mut ops = vec![];
	let mut doc = (**crdt).clone();
	let char_range = get_char_range(&op);
	let utf8_range = to_utf8_range(&doc, &char_range);
	match op.kind {
		OpKind::Ins => {
			debug_assert!(op.loc.fwd);
			debug_assert_lt!(op.loc.span.start, op.loc.span.end);
			for (i, x) in op.content.as_ref().unwrap().as_bytes().iter().enumerate() {
				let ins = doc.insert(utf8_range.0.start + i, *x);
				doc.apply(&ins);
				ops.push(ins);
			}
		}
		OpKind::Del => {
			for _ in 0..utf8_range.0.len() {
				let del = doc.delete(utf8_range.0.start);
				doc.apply(&del);
				ops.push(del);
			}
		}
	}
	ops
}

pub fn replicate_random_change<const VERBOSE: bool>(crdt: &mut Crdt<List<u8>>, prev_oplog: &OpLog, curr_oplog: &OpLog) {
	let idx = diff_first_idx(&prev_oplog, &curr_oplog);
	if VERBOSE && idx.is_some() {
		println!("diamond types collapsed last operation");
	}

	// last operation previously in the oplog may have been collapsed
	let n_undos = idx.is_some() as usize;
	let undos = last_n_ops(&prev_oplog, n_undos).rev().map(|op| not(op));

	let n_dos = curr_oplog.operations.0.len() - idx.unwrap_or_else(|| prev_oplog.operations.0.len());
	let dos = last_n_ops(&curr_oplog, n_dos);

	for op in undos.chain(dos) {
		if VERBOSE {
			println!("{op:?}");
		}
		let instrs = convert(crdt, &op);
		for instr in instrs {
			crdt.apply_(instr);
		}
	}
}

/// Checks if strings are the same two sub-strings appended in different (or same) order
pub fn check_two_substrings(self_: &String, other: &String) -> bool {
	if self_.is_empty() && other.is_empty() {
		return true;
	}

	let self_chars: Vec<_> = self_.chars().collect();
	let other_chars: Vec<_> = other.chars().collect();

	if self_chars.len() != other_chars.len() {
		return false;
	}

	for i in 0..self_chars.len() {
		let (self_lhs, self_rhs) = self_chars.split_at(i);
		let (other_lhs, other_rhs) = other_chars.split_at(self_chars.len() - i);
		if self_lhs == other_rhs && self_rhs == other_lhs {
			return true;
		}
	}

	false
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_check_two_substrings() {
		assert!(check_two_substrings(&String::from(""), &String::from("")));
		assert!(check_two_substrings(&String::from("A"), &String::from("A")));
		assert!(check_two_substrings(&String::from("Alec"), &String::from("Alec")));
		assert!(check_two_substrings(&String::from("AlecAlex"), &String::from("AlexAlec")));
		assert!(check_two_substrings(&String::from("AlecAlexander"), &String::from("AlexanderAlec")));
		assert!(check_two_substrings(&String::from("𐆚"), &String::from("𐆚")));
		assert!(check_two_substrings(&String::from("δ𐆔𐆚"), &String::from("𐆚δ𐆔")));
		assert!(!check_two_substrings(&String::from("A"), &String::from("G")));
		assert!(!check_two_substrings(&String::from("Alec"), &String::from("Giovanni")));
		assert!(!check_two_substrings(&String::from("Alec"), &String::from("Alex")));
		assert!(!check_two_substrings(&String::from("AlecxelA"), &String::from("AlexcelA")));
		assert!(!check_two_substrings(&String::from("AlecrednaxelA"), &String::from("AlexandercelA")));
		assert!(!check_two_substrings(&String::from("δ𐆔𐆚"), &String::from("δ𐆚𐆔")));
	}
}
