#![allow(clippy::if_not_else, clippy::range_plus_one)]

use itertools::Itertools;
use rand::{prelude::SliceRandom, rngs::SmallRng, Rng, SeedableRng};
use random_branch::branch_using;

use otto::{list::List, settable::Settable as Register, StateTest};

use otto_test::{channel::channel, crdt_client::CrdtClient};

fn test_crdt<T: StateTest>(rng: &mut impl Rng) {
	let clients = 5;
	let mut iters = 16usize;
	let start = T::gen(rng);
	let channels = (0..clients).map(|_| channel()).collect::<Vec<_>>();
	let mut clients = channels
		.iter()
		.enumerate()
		.map(|(i, (_, inbox))| {
			CrdtClient::new(
				start.clone(),
				inbox.clone(),
				channels.iter().enumerate().filter_map(|(i_, (outbox, _))| (i != i_).then(|| outbox.clone())),
			)
		})
		.collect::<Vec<_>>();
	while iters != 0 || !clients.iter().all(CrdtClient::drained) {
		loop {
			break branch_using!(*rng, {
				{
					if iters == 0 {
						continue;
					}
					let client = clients.choose_mut(rng).unwrap();
					client.gen_and_send(rng);
				},
				{
					let client = clients.choose_mut(rng).unwrap();
					if !client.try_recv_and_commit() {
						continue;
					}
				},
			});
		}
		iters = iters.saturating_sub(1);
	}
	assert!(clients.iter().map(CrdtClient::state).all_equal());
}

#[ignore]
#[test]
fn fuzz_crdt() {
	let seed = rand::random();
	println!("seed: {seed}");
	let rng = &mut SmallRng::seed_from_u64(seed);
	for i in 0..u64::MAX {
		if i % 1_000 == 0 {
			println!("{}", i);
		}
		test_crdt::<List<List<Register<u64>>>>(rng);
	}
}

#[test]
fn fuzz_crdt_short() {
	let seed = rand::random();
	println!("seed: {seed}");
	let rng = &mut SmallRng::seed_from_u64(seed);
	for _ in 0..100 {
		test_crdt::<List<List<Register<u64>>>>(rng);
	}
}
