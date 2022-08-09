use rand::Rng;
use std::collections::VecDeque;

use otto::{State, StateTest};

use crate::channel::{Receiver, Sender};

#[derive(Debug)]
pub struct OtClient<T>
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
	pub fn new(state: T, from_server: Receiver<Option<T::Instr>>, to_server: Sender<Option<T::Instr>>) -> Self {
		Self { state, pending: VecDeque::new(), from_server, to_server }
	}
	pub fn gen_and_send(&mut self, rng: &mut impl Rng) {
		let instr = StateTest::gen_trivial_instr(&self.state, rng).unwrap();
		self.state.apply(&instr);
		let instr_clone = instr.clone();
		let instr = T::insert_and_rebase_back(instr, &*self.pending.make_contiguous());
		self.pending.push_back(instr_clone);
		self.to_server.send(Some(instr));
	}
	pub fn try_recv_and_commit(&mut self) -> bool {
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
	pub fn state(&self) -> &T {
		&self.state
	}
	pub fn drained(&self) -> bool {
		self.pending.is_empty() && self.from_server.is_empty() && self.to_server.is_empty()
	}
}
