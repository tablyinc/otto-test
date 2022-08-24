use std::ops::Range;

use all_asserts::{assert_lt, debug_assert_lt};
use diamond_types::{
	list::{
		operation::{OpKind, Operation}, OpLog
	}, Time
};
use otto::{
	crdt::Crdt, list_wrap::{ListInstrWrap, ListWrap}, State
};
use rand::{random, rngs::SmallRng, Rng, SeedableRng};

struct CharRange(Range<usize>);

struct Utf8Range(Range<usize>);

pub(crate) fn doc_to_string(doc: &ListWrap<u8>) -> String {
	String::from_utf8((0..doc.0.len()).map(|at| doc.0[at]).collect::<Vec<_>>()).unwrap()
}

fn get_char_range(op: &Operation) -> CharRange {
	CharRange(Range { start: op.loc.span.start.min(op.loc.span.end), end: op.loc.span.start.max(op.loc.span.end) })
}

fn to_utf8_range(doc: &ListWrap<u8>, char_range: &CharRange) -> Utf8Range {
	let string = doc_to_string(&doc);
	let offset = string.chars().take(char_range.0.start).map(|char| char.len_utf8()).sum();
	let span: usize = string.chars().skip(char_range.0.start).take(char_range.0.end - char_range.0.start).map(|char| char.len_utf8()).sum();
	Utf8Range(offset..offset + span)
}

fn convert(crdt: &Crdt<ListWrap<u8>>, op: &Operation) -> Vec<ListInstrWrap<u8>> {
	debug_assert!(op.content.is_some());
	let mut ops = vec![];
	let mut doc = (**crdt).clone();
	let char_range = get_char_range(&op);
	let utf8_range = to_utf8_range(&doc, &char_range);
	let rng = &mut SmallRng::seed_from_u64(random());
	match op.kind {
		OpKind::Ins => {
			debug_assert!(op.loc.fwd);
			debug_assert_lt!(op.loc.span.start, op.loc.span.end);
			for (i, x) in op.content.as_ref().unwrap().as_bytes().iter().enumerate() {
				let ins = doc.0.insert(utf8_range.0.start + i, *x);
				doc.0.apply(&ins);
				ops.push(ListInstrWrap(ins, rng.gen()));
			}
		}
		OpKind::Del => {
			for _ in 0..utf8_range.0.len() {
				let del = doc.0.delete(utf8_range.0.start);
				doc.0.apply(&del);
				ops.push(ListInstrWrap(del, rng.gen()));
			}
		}
	}
	ops
}

pub(crate) fn replicate_random_change<const VERBOSE: bool>(crdt: &mut Crdt<ListWrap<u8>>, prev_version: &[Time], curr_oplog: &OpLog) {
	for op in curr_oplog.iter_range_since(prev_version) {
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
pub(crate) fn check_two_substrings(self_: &String, other: &String) -> bool {
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
