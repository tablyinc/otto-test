#![allow(clippy::if_not_else, clippy::range_plus_one)]

use itertools::{multizip as zip, Itertools};
use rand::{prelude::SliceRandom, rngs::SmallRng, Rng, SeedableRng};
use random_branch::branch_using;

use otto::{crdt::Crdt, list::List, register::Register, text::Text, StateTest};

use otto_test::bridge::CrdtClientOtServer;
use otto_test::{
    channel::channel, crdt_client::CrdtClient, ot_client::OtClient, ot_server::OtServer,
};

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn test_crdt_ot<T: StateTest>(rng: &mut impl Rng) {
    let clients_crdt = 5;
    let clients_ot = 5;
    let mut iters = 50usize;
    let start = T::gen(rng);

    let (to_client, from_server): (Vec<_>, Vec<_>) = (0..clients_ot + 1)
        .map(|_| channel::<Option<T::Instr>>())
        .multiunzip();
    let (to_server, from_client): (Vec<_>, Vec<_>) = (0..clients_ot + 1)
        .map(|_| channel::<Option<T::Instr>>())
        .multiunzip();
    let crdt_channels = (0..clients_crdt + 1).map(|_| channel()).collect::<Vec<_>>();
    let mut server = CrdtClientOtServer::<T>::new(
        start.clone(),
        crdt_channels.last().unwrap().1.clone(),
        crdt_channels[..crdt_channels.len() - 1]
            .iter()
            .map(|channel| channel.0.clone()),
        zip((to_client, from_client)),
    );
    let mut clients_ot = zip((from_server, to_server))
        .map(|(from_server, to_server)| OtClient::new(start.clone(), from_server, to_server))
        .collect::<Vec<_>>();

    let mut clients_crdt = crdt_channels[..crdt_channels.len() - 1]
        .iter()
        .enumerate()
        .map(|(i, (_, inbox))| {
            CrdtClient::new(
                start.clone(),
                inbox.clone(),
                crdt_channels
                    .iter()
                    .enumerate()
                    .filter_map(|(i_, (outbox, _))| (i != i_).then(|| outbox.clone())),
            )
        })
        .collect::<Vec<_>>();

    while iters != 0
        || !clients_crdt.iter().all(CrdtClient::drained)
        || !server.drained()
        || !clients_ot.iter().all(OtClient::drained)
    {
        loop {
            break branch_using!(*rng, {
                {
                    if iters == 0 {
                        continue;
                    }
                    let client = clients_crdt.choose_mut(rng).unwrap();
                    client.gen_and_send(rng);
                },
                {
                    let client = clients_crdt.choose_mut(rng).unwrap();
                    if !client.try_recv_and_commit() {
                        continue;
                    }
                },

                {
                    if iters == 0 {
                        continue;
                    }
                    server.gen_and_send(rng);
                },
                {
                    if !server.try_recv_and_send(rng) {
                        continue;
                    }
                },
                {
                    if !server.try_recv_and_commit() {
                        continue;
                    }
                },

                {
                    if iters == 0 {
                        continue;
                    }
                    let client = clients_ot.choose_mut(rng).unwrap();
                    client.gen_and_send(rng);
                },
                {
                    let client = clients_ot.choose_mut(rng).unwrap();
                    if !client.try_recv_and_commit() {
                        continue;
                    }
                },
            });
        }
        iters = iters.saturating_sub(1);
    }
    assert!(clients_crdt
        .iter()
        .map(CrdtClient::state)
        .chain(clients_ot.iter().map(OtClient::state))
        .all_equal());
}

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
                channels
                    .iter()
                    .enumerate()
                    .filter_map(|(i_, (outbox, _))| (i != i_).then(|| outbox.clone())),
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

    for _ in 0..rng.gen_range(0..100) {
        let upstream_instr = StateTest::gen_trivial_instr(&*upstream_crdt, rng).unwrap();
        let crdt_instr = upstream_crdt.instr_to_crdt_instr(upstream_instr.clone());
        upstream_crdt.apply_(upstream_instr.clone());

        let downstream_instr = downstream_crdt.instr_from_crdt_instr_(crdt_instr);
        downstream_crdt.apply_(downstream_instr.clone());

        // println!("dd'd: {:?} -> {:?}", upstream_instr, downstream_instr);
    }
}

fn test_ot<T: StateTest>(rng: &mut impl Rng) {
    let clients = 5;
    let mut iters = 100usize;
    let start = T::gen(rng);
    let (to_client, from_server): (Vec<_>, Vec<_>) = (0..clients)
        .map(|_| channel::<Option<T::Instr>>())
        .multiunzip();
    let (to_server, from_client): (Vec<_>, Vec<_>) = (0..clients)
        .map(|_| channel::<Option<T::Instr>>())
        .multiunzip();
    let mut clients = zip((from_server, to_server))
        .map(|(from_server, to_server)| OtClient::new(start.clone(), from_server, to_server))
        .collect::<Vec<_>>();
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
fn test_client_server_mock() {
    let seed = rand::random();
    println!("seed: {seed}");
    let rng = &mut SmallRng::seed_from_u64(seed);
    for i in 0..u64::MAX {
        if i % 1_000 == 0 {
            println!("{}", i);
        }
        test_crdt::<List<List<Register<u64>>>>(rng);
        test_crdt_ot::<List<List<Register<u8>>>>(rng);
        test_ot::<List<Text>>(rng);
        test_ot::<List<List<Register<u64>>>>(rng);
        test_crdt_differential_dataflow::<List<List<Register<u8>>>>(rng);
    }
}

#[test]
fn test_client_server_mock_short() {
    let seed = rand::random();
    println!("seed: {seed}");
    let rng = &mut SmallRng::seed_from_u64(seed);
    for _ in 0..100 {
        test_crdt::<List<List<Register<u64>>>>(rng);
        test_crdt_ot::<List<List<Register<u8>>>>(rng);
        test_ot::<List<List<Register<u64>>>>(rng);
        test_crdt_differential_dataflow::<List<List<Register<u8>>>>(rng);
    }
}
