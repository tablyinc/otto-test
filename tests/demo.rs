#![feature(let_else)]
#![allow(clippy::if_not_else, clippy::range_plus_one)]

use itertools::{multizip as zip, Itertools};
use rand::{prelude::SliceRandom, rngs::SmallRng, seq::IteratorRandom, Rng, SeedableRng};
use random_branch::branch_using;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use otto::{
    crdt::{Crdt, CrdtInstr},
    list::List,
    register::Register,
    text::Text,
    State, StateTest,
};

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let channel = Rc::new(RefCell::new(VecDeque::new()));
    (Sender(channel.clone()), Receiver(channel))
}
#[derive(Clone)]
struct Sender<T>(Rc<RefCell<VecDeque<T>>>);
#[derive(Clone)]
struct Receiver<T>(Rc<RefCell<VecDeque<T>>>);
impl<T> Sender<T> {
    fn len(&self) -> usize {
        self.0.borrow().len()
    }
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn send(&self, val: T) {
        self.0.borrow_mut().push_back(val);
    }
}
impl<T> Receiver<T> {
    fn len(&self) -> usize {
        self.0.borrow().len()
    }
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn try_receive(&self) -> Option<T> {
        self.0.borrow_mut().pop_front()
    }
}

struct OtServer<T>
where
    T: State,
{
    pending: VecDeque<T::Instr>,
    clients: Vec<OtServerClient<T>>,
}
struct OtServerClient<T>
where
    T: State,
{
    offset: usize,
    to_client: Sender<Option<T::Instr>>,
    from_client: Receiver<Option<T::Instr>>,
}
impl<T> OtServer<T>
where
    T: State,
{
    fn new(
        channels: impl Iterator<Item = (Sender<Option<T::Instr>>, Receiver<Option<T::Instr>>)>,
    ) -> Self {
        Self {
            pending: VecDeque::new(),
            clients: channels
                .map(|(to_client, from_client)| OtServerClient::new(to_client, from_client))
                .collect(),
        }
    }
    fn try_recv_and_send(&mut self, rng: &mut impl Rng) -> bool {
        if let Some((client, OtServerClient { from_client, .. })) = self
            .clients
            .iter_mut()
            .enumerate()
            .filter(|(_, OtServerClient { from_client, .. })| !from_client.is_empty())
            .choose(rng)
        {
            match from_client.try_receive().unwrap() {
                Some(instr) => {
                    let offset = self.clients[client].offset;
                    let instr = T::insert_and_rebase_forward(
                        instr,
                        &self.pending.make_contiguous()[offset..],
                    );
                    self.pending.push_back(instr.clone());
                    self.clients.iter_mut().enumerate().for_each(
                        |(client_, OtServerClient { to_client, .. })| {
                            to_client.send(if client_ != client {
                                Some(instr.clone())
                            } else {
                                None
                            });
                        },
                    );
                }
                None => {
                    self.clients[client].offset += 1;
                    let x = self
                        .clients
                        .iter()
                        .map(|&OtServerClient { offset, .. }| offset)
                        .min()
                        .unwrap();
                    for _ in 0..x {
                        let _ = self.pending.pop_front().unwrap();
                    }
                    self.clients
                        .iter_mut()
                        .for_each(|OtServerClient { offset, .. }| *offset -= x);
                }
            }
            true
        } else {
            false
        }
    }
    fn drained(&self) -> bool {
        self.pending.is_empty()
    }
}
impl<T> OtServerClient<T>
where
    T: State,
{
    fn new(to_client: Sender<Option<T::Instr>>, from_client: Receiver<Option<T::Instr>>) -> Self {
        Self {
            offset: 0,
            to_client,
            from_client,
        }
    }
}

struct OtClient<T>
where
    T: State,
{
    state: T,
    pending: VecDeque<T::Instr>,
    from_server: Receiver<Option<T::Instr>>,
    to_server: Sender<Option<T::Instr>>,
}
impl<T> OtClient<T>
where
    T: StateTest,
{
    fn new(
        state: T,
        from_server: Receiver<Option<T::Instr>>,
        to_server: Sender<Option<T::Instr>>,
    ) -> Self {
        Self {
            state,
            pending: VecDeque::new(),
            from_server,
            to_server,
        }
    }
    fn gen_and_send(&mut self, rng: &mut impl Rng) {
        let instr = StateTest::gen_trivial_instr(&self.state, rng).unwrap();
        self.state.apply(&instr);
        let instr_clone = instr.clone();
        let instr = T::insert_and_rebase_back(instr, &*self.pending.make_contiguous());
        self.pending.push_back(instr_clone);
        self.to_server.send(Some(instr));
    }
    fn try_recv_and_commit(&mut self) -> bool {
        if let Some(instr) = self.from_server.try_receive() {
            match instr {
                Some(instr) => {
                    let instr = T::converge(instr, self.pending.make_contiguous());
                    self.state.apply(&instr);
                }
                None => {
                    let _instr = self.pending.pop_front().unwrap();
                }
            }
            self.to_server.send(None);
            true
        } else {
            false
        }
    }
    fn drained(&self) -> bool {
        self.pending.is_empty() && self.from_server.is_empty() && self.to_server.is_empty()
    }
}

struct CrdtClient<T>
where
    T: State,
{
    crdt: Crdt<T>,
    inbox: Receiver<CrdtInstr<T>>,
    outboxes: Vec<Sender<CrdtInstr<T>>>,
}
impl<T> CrdtClient<T>
where
    T: StateTest,
{
    fn new(
        state: T,
        inbox: Receiver<CrdtInstr<T>>,
        outboxes: impl Iterator<Item = Sender<CrdtInstr<T>>>,
    ) -> Self {
        Self {
            crdt: Crdt::new(state),
            inbox,
            outboxes: outboxes.collect(),
        }
    }
    fn gen_and_send(&mut self, rng: &mut impl Rng) {
        let instr = if self.crdt.instrs().len() == 0 || rng.gen() {
            let instr = StateTest::gen_trivial_instr(&*self.crdt, rng).unwrap();
            Crdt::instr_to_crdt_instr(&self.crdt, instr)
        } else {
            let mut undos = self.crdt.instrs();
            let undo = rng.gen_range(0..undos.len());
            undos.nth(undo).unwrap().inverse()
        };
        self.crdt.apply(instr.clone());
        self.outboxes
            .iter_mut()
            .for_each(|outbox| outbox.send(instr.clone()));
    }
    fn try_recv_and_commit(&mut self) -> bool {
        if let Some(instr) = self.inbox.try_receive() {
            self.crdt.apply(instr);
            true
        } else {
            false
        }
    }
    fn drained(&self) -> bool {
        self.inbox.is_empty()
    }
}

struct CrdtClientOtServer<T>
where
    T: State,
{
    crdt: CrdtClient<T>,
    ot: OtServer<T>,
}
impl<T> CrdtClientOtServer<T>
where
    T: StateTest,
{
    fn new(
        state: T,
        inbox: Receiver<CrdtInstr<T>>,
        outboxes: impl Iterator<Item = Sender<CrdtInstr<T>>>,
        channels: impl Iterator<Item = (Sender<Option<T::Instr>>, Receiver<Option<T::Instr>>)>,
    ) -> Self {
        Self {
            crdt: CrdtClient::new(state, inbox, outboxes),
            ot: OtServer::new(channels),
        }
    }
    fn gen_and_send(&mut self, rng: &mut impl Rng) {
        let crdt_instr = if self.crdt.crdt.instrs().len() == 0 || rng.gen() {
            let ot_instr = StateTest::gen_trivial_instr(&*self.crdt.crdt, rng).unwrap();
            Crdt::instr_to_crdt_instr(&self.crdt.crdt, ot_instr)
        } else {
            let mut undos = self.crdt.crdt.instrs();
            let undo = rng.gen_range(0..undos.len());
            undos.nth(undo).unwrap().inverse()
        };
        self.crdt
            .outboxes
            .iter_mut()
            .for_each(|outbox| outbox.send(crdt_instr.clone()));

        let ot_instr = self.crdt.crdt.instr_from_crdt_instr_(crdt_instr.clone());
        self.ot.pending.push_back(ot_instr.clone());
        self.ot
            .clients
            .iter_mut()
            .for_each(|OtServerClient { to_client, .. }| {
                to_client.send(Some(ot_instr.clone()));
            });

        self.crdt.crdt.apply(crdt_instr);
    }
    fn try_recv_and_commit(&mut self) -> bool {
        let Some(crdt_instr) = self.crdt.inbox.try_receive() else { return false };
        let ot_instr = self.crdt.crdt.instr_from_crdt_instr_(crdt_instr.clone());
        self.ot.pending.push_back(ot_instr.clone());
        self.ot
            .clients
            .iter_mut()
            .for_each(|OtServerClient { to_client, .. }| {
                to_client.send(Some(ot_instr.clone()));
            });

        self.crdt.crdt.apply(crdt_instr);

        true
    }
    fn try_recv_and_send(&mut self, rng: &mut impl Rng) -> bool {
        let Some((client, OtServerClient { from_client, .. })) = self.ot.clients.iter_mut().enumerate().filter(|(_, OtServerClient { from_client, .. })| !from_client.is_empty()).choose(rng) else { return false };
        if let Some(ot_instr) = from_client.try_receive().unwrap() {
            let offset = self.ot.clients[client].offset;
            let ot_instr = T::insert_and_rebase_forward(
                ot_instr,
                &self.ot.pending.make_contiguous()[offset..],
            );

            let crdt_instr = self.crdt.crdt.instr_to_crdt_instr(ot_instr.clone());
            self.crdt
                .outboxes
                .iter_mut()
                .for_each(|outbox| outbox.send(crdt_instr.clone()));

            self.ot.pending.push_back(ot_instr.clone());
            self.ot.clients.iter_mut().enumerate().for_each(
                |(client_, OtServerClient { to_client, .. })| {
                    to_client.send(if client_ != client {
                        Some(ot_instr.clone())
                    } else {
                        None
                    });
                },
            );

            self.crdt.crdt.apply(crdt_instr);
        } else {
            self.ot.clients[client].offset += 1;
            let x = self
                .ot
                .clients
                .iter()
                .map(|&OtServerClient { offset, .. }| offset)
                .min()
                .unwrap();
            for _ in 0..x {
                let _ = self.ot.pending.pop_front().unwrap();
            }
            self.ot
                .clients
                .iter_mut()
                .for_each(|OtServerClient { offset, .. }| *offset -= x);
        }
        true
    }
    fn drained(&self) -> bool {
        self.crdt.drained() && self.ot.drained()
    }
}

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
        .map(|client| &*client.crdt)
        .chain(clients_ot.iter().map(|client| &client.state))
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
    assert!(clients.iter().map(|client| &*client.crdt).all_equal());
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
    assert!(clients.iter().map(|client| &client.state).all_equal());
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
