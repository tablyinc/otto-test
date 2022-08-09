use rand::{seq::IteratorRandom, Rng};

use otto::{
	crdt::{Crdt, CrdtInstr}, State, StateTest
};

use crate::{
	channel::{Receiver, Sender}, crdt_client::CrdtClient, ot_server::{OtServer, OtServerClient}
};

#[derive(Debug)]
pub struct CrdtClientOtServer<T>
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
	pub fn new(
		state: T, inbox: Receiver<CrdtInstr<T>>, outboxes: impl Iterator<Item = Sender<CrdtInstr<T>>>,
		channels: impl Iterator<Item = (Sender<Option<T::Instr>>, Receiver<Option<T::Instr>>)>,
	) -> Self {
		Self { crdt: CrdtClient::new(state, inbox, outboxes), ot: OtServer::new(channels) }
	}
	pub fn gen_and_send(&mut self, rng: &mut impl Rng) {
		let crdt_instr = if self.crdt.crdt.instrs().len() == 0 || rng.gen_range(0..5) != 0 {
			let ot_instr = StateTest::gen_trivial_instr(&*self.crdt.crdt, rng).unwrap();
			Crdt::instr_to_crdt_instr(&self.crdt.crdt, ot_instr)
		} else {
			let mut undos = self.crdt.crdt.instrs();
			let undo = rng.gen_range(0..undos.len());
			undos.nth(undo).unwrap().inverse()
		};
		self.crdt.outboxes.iter_mut().for_each(|outbox| outbox.send(crdt_instr.clone()));

		let ot_instr = self.crdt.crdt.instr_from_crdt_instr_(crdt_instr.clone());
		self.ot.pending.push_back(ot_instr.clone());
		self.ot.clients.iter_mut().for_each(|OtServerClient { to_client, .. }| {
			to_client.send(Some(ot_instr.clone()));
		});

		self.crdt.crdt.apply(crdt_instr);
	}
	pub fn try_recv_and_commit(&mut self) -> bool {
		let Some(crdt_instr) = self.crdt.inbox.try_receive() else { return false };
		let ot_instr = self.crdt.crdt.instr_from_crdt_instr_(crdt_instr.clone());
		self.ot.pending.push_back(ot_instr.clone());
		self.ot.clients.iter_mut().for_each(|OtServerClient { to_client, .. }| {
			to_client.send(Some(ot_instr.clone()));
		});

		self.crdt.crdt.apply(crdt_instr);

		true
	}
	pub fn try_recv_and_send(&mut self, rng: &mut impl Rng) -> bool {
		let Some((client, OtServerClient { from_client, .. })) = self.ot.clients.iter_mut().enumerate().filter(|(_, OtServerClient { from_client, .. })| !from_client.is_empty()).choose(rng) else { return false };
		if let Some(ot_instr) = from_client.try_receive().unwrap() {
			let offset = self.ot.clients[client].offset;
			let ot_instr = T::insert_and_rebase_forward(ot_instr, &self.ot.pending.make_contiguous()[offset..]);

			let crdt_instr = self.crdt.crdt.instr_to_crdt_instr(ot_instr.clone());
			self.crdt.outboxes.iter_mut().for_each(|outbox| outbox.send(crdt_instr.clone()));

			self.ot.pending.push_back(ot_instr.clone());
			self.ot.clients.iter_mut().enumerate().for_each(|(client_, OtServerClient { to_client, .. })| {
				to_client.send(if client_ != client { Some(ot_instr.clone()) } else { None });
			});

			self.crdt.crdt.apply(crdt_instr);
		} else {
			self.ot.clients[client].offset += 1;
			let x = self.ot.clients.iter().map(|&OtServerClient { offset, .. }| offset).min().unwrap();
			for _ in 0..x {
				let _ = self.ot.pending.pop_front().unwrap();
			}
			self.ot.clients.iter_mut().for_each(|OtServerClient { offset, .. }| *offset -= x);
		}
		true
	}
	pub fn drained(&self) -> bool {
		self.crdt.drained() && self.ot.drained()
	}
}
