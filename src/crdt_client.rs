use otto::{
	crdt::{Crdt, CrdtInstr}, State, StateTest
};
use rand::Rng;

use crate::channel::{Receiver, Sender};

#[derive(Debug)]
pub struct CrdtClient<T>
where
	T: State,
{
	pub(crate) crdt: Crdt<T>,
	pub(crate) inbox: Receiver<CrdtInstr<T>>,
	pub(crate) outboxes: Vec<Sender<CrdtInstr<T>>>,
}

impl<T> CrdtClient<T>
where
	T: StateTest,
{
	pub fn new(state: T, inbox: Receiver<CrdtInstr<T>>, outboxes: impl Iterator<Item = Sender<CrdtInstr<T>>>) -> Self {
		Self { crdt: Crdt::new(state), inbox, outboxes: outboxes.collect() }
	}
	pub fn gen_and_send(&mut self, rng: &mut impl Rng) {
		let instr = if self.crdt.instrs().len() == 0 || rng.gen_range(0..5) != 0 {
			let instr = StateTest::gen_trivial_instr(&*self.crdt, rng).unwrap();
			Crdt::instr_to_crdt_instr(&self.crdt, instr)
		} else {
			let mut undos = self.crdt.instrs();
			let undo = rng.gen_range(0..undos.len());
			undos.nth(undo).unwrap().inverse()
		};
		self.crdt.apply(instr.clone());
		self.outboxes.iter_mut().for_each(|outbox| outbox.send(instr.clone()));
	}
	pub fn try_recv_and_commit(&mut self) -> bool {
		if let Some(instr) = self.inbox.try_receive() {
			self.crdt.apply(instr);
			true
		} else {
			false
		}
	}
	pub fn state(&self) -> &T {
		&self.crdt
	}
	pub fn drained(&self) -> bool {
		self.inbox.is_empty()
	}
}
