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

use std::{alloc, str};

use all_asserts::assert_le;
use cap::Cap;
use crdt_testdata::{load_testing_data, TestData, TestPatch};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main, Throughput};
use otto::{crdt::Crdt, list::{List, OttoList}};

#[global_allocator]
static ALLOCATOR: Cap<alloc::System> = Cap::new(alloc::System, usize::MAX);

// TODO support all datasets
const DATASETS: &[&str] = &[
	"automerge-paper",
	// "rustcode",
	"sveltecomponent",
	// "seph-blog1",
];

const CRITERION_MIN_SAMPLE_SIZE: usize = 10;

fn testing_data(name: &str) -> TestData {
	let filename = format!("benchmark_data/{}.json.gz", name);
	load_testing_data(&filename)
}

fn apply_doc(test_data: &TestData) -> Crdt<List<u8>> {
	let mut doc = <Crdt<_>>::new(List::new());

	for txn in &test_data.txns {
		for TestPatch(pos, del_span, ins_content) in &txn.patches {
			for _ in 0..*del_span {
				let instr = doc.delete(*pos);
				doc.apply_(instr);
			}

			for (i, x) in ins_content.as_bytes().iter().enumerate() {
				let instr = doc.insert(*pos + i, *x);
				doc.apply_(instr);
			}
		}
	}
	debug_assert_eq!(test_data.end_content.len(), doc.len());
	debug_assert_eq!(test_data.end_content, doc_to_string(&doc));
	doc
}

fn doc_to_string(doc: &List<u8>) -> String {
	String::from_utf8((0..doc.len()).map(|at| doc[at]).collect::<Vec<_>>()).unwrap()
}

fn local_benchmarks(c: &mut Criterion) {
	for name in DATASETS {
		let test_data = testing_data(name);
		println!("{name}");
		println!("no. operations: {}", test_data.len());
		assert_eq!(test_data.start_content.len(), 0);
		println!("document chars: {}", test_data.end_content.len());
		assert_le!(test_data.end_content.len(), test_data.len());

		let mut doc = <Crdt<_>>::new(List::new());
		let mut group = c.benchmark_group("local");
		group.sample_size(CRITERION_MIN_SAMPLE_SIZE);
		group.throughput(Throughput::Elements(test_data.len() as u64));
		group.bench_function(BenchmarkId::new("apply", name), |b| {
			b.iter(|| doc = apply_doc(&test_data));
		});
		group.finish();
		println!("Currently allocated: {}B", ALLOCATOR.allocated());
		println!();
	}
}

criterion_group!(benches, local_benchmarks);
criterion_main!(benches);
