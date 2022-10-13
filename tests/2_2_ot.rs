#![allow(clippy::if_not_else, clippy::range_plus_one)]

use itertools::{Itertools, multizip as zip};
use otto::{list::List, mappable_register::MappableRegister, StateTest, text::Text};
use rand::{prelude::SliceRandom, Rng, rngs::SmallRng, SeedableRng};
use random_branch::branch_using;

use otto_test::{channel::channel, ot_client::OtClient, ot_server::OtServer};

fn test_ot<T: StateTest>(rng: &mut impl Rng) {
	let clients = 5;
	let mut iters = 100usize;
	let start = T::gen(rng);
	let (to_client, from_server): (Vec<_>, Vec<_>) = (0..clients).map(|_| channel::<Option<T::Instr>>()).multiunzip();
	let (to_server, from_client): (Vec<_>, Vec<_>) = (0..clients).map(|_| channel::<Option<T::Instr>>()).multiunzip();
	let mut clients =
		zip((from_server, to_server)).map(|(from_server, to_server)| OtClient::new(start.clone(), from_server, to_server)).collect::<Vec<_>>();
	let mut server = OtServer::<T>::new(zip((to_client, from_client)));
	while iters != 0 || !clients.iter().all(OtClient::drained) {
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
					if !server.try_recv_and_send(rng) {
						continue;
					}
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
	assert!(clients.iter().map(OtClient::state).all_equal());
	assert!(server.drained());
}

#[ignore]
#[test]
fn fuzz_ot() {
	let seed = rand::random();
	println!("seed: {seed}");
	let rng = &mut SmallRng::seed_from_u64(seed);
	for i in 0..u64::MAX {
		if i % 1_000 == 0 {
			println!("{}", i);
		}
		test_ot::<Text>(rng);
		test_ot::<List<List<MappableRegister<u64>>>>(rng);
	}
}

#[test]
fn fuzz_ot_short() {
	let seed = rand::random();
	println!("seed: {seed}");
	let rng = &mut SmallRng::seed_from_u64(seed);
	for _ in 0..100 {
		test_ot::<Text>(rng);
		test_ot::<List<List<MappableRegister<u64>>>>(rng);
	}
}
