use std::ops::Range;

use all_asserts::{assert_lt, debug_assert_lt};
use diamond_types::{
	list::{
		operation::{OpKind, Operation}, OpLog
	}, Time
};
use otto::{
	crdt::Crdt, list::{List, ListInstr}, State
};

struct CharRange(Range<usize>);

struct Utf8Range(Range<usize>);

pub(crate) fn doc_to_string(doc: &List<u8>) -> String {
	String::from_utf8((0..doc.len()).map(|at| doc[at]).collect::<Vec<_>>()).unwrap()
}

fn get_char_range(op: &Operation) -> CharRange {
	CharRange(Range { start: op.loc.span.start.min(op.loc.span.end), end: op.loc.span.start.max(op.loc.span.end) })
}

fn to_utf8_range(doc: &List<u8>, char_range: &CharRange) -> Utf8Range {
	let string = doc_to_string(doc);
	let offset = string.chars().take(char_range.0.start).map(|char| char.len_utf8()).sum();
	let span: usize = string.chars().skip(char_range.0.start).take(char_range.0.end - char_range.0.start).map(|char| char.len_utf8()).sum();
	Utf8Range(offset..offset + span)
}

fn convert(crdt: &Crdt<List<u8>>, op: &Operation) -> Vec<ListInstr<u8>> {
	debug_assert!(op.content.is_some());
	let mut ops = vec![];
	let mut doc = (**crdt).clone();
	let char_range = get_char_range(op);
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

pub(crate) fn replicate_random_change<const VERBOSE: bool>(crdt: &mut Crdt<List<u8>>, prev_version: &[Time], curr_oplog: &OpLog) {
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
