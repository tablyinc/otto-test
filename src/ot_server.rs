use std::collections::VecDeque;

use otto::State;
use rand::{seq::IteratorRandom, Rng};

use crate::channel::{Receiver, Sender};

#[derive(Debug)]
pub struct OtServer<T>
where
	T: State,
{
	pub(crate) pending: VecDeque<T::Instr>,
	pub(crate) clients: Vec<OtServerClient<T>>,
}

#[derive(Debug)]
pub(crate) struct OtServerClient<T>
where
	T: State,
{
	pub(crate) offset: usize,
	pub(crate) to_client: Sender<Option<T::Instr>>,
	pub(crate) from_client: Receiver<Option<T::Instr>>,
}

impl<T> OtServer<T>
where
	T: State,
{
	pub fn new(channels: impl Iterator<Item = (Sender<Option<T::Instr>>, Receiver<Option<T::Instr>>)>) -> Self {
		Self { pending: VecDeque::new(), clients: channels.map(|(to_client, from_client)| OtServerClient::new(to_client, from_client)).collect() }
	}
	pub fn try_recv_and_send(&mut self, rng: &mut impl Rng) -> bool {
		if let Some((client, OtServerClient { from_client, .. })) =
			self.clients.iter_mut().enumerate().filter(|(_, OtServerClient { from_client, .. })| !from_client.is_empty()).choose(rng)
		{
			match from_client.try_receive().unwrap() {
				Some(instr) => {
					let offset = self.clients[client].offset;
					let instr = T::insert_and_rebase_forward(instr, &self.pending.make_contiguous()[offset..]);
					self.pending.push_back(instr.clone());
					self.clients.iter_mut().enumerate().for_each(|(client_, OtServerClient { to_client, .. })| {
						to_client.send(if client_ != client { Some(instr.clone()) } else { None });
					});
				}
				None => {
					self.clients[client].offset += 1;
					let x = self.clients.iter().map(|&OtServerClient { offset, .. }| offset).min().unwrap();
					for _ in 0..x {
						let _ = self.pending.pop_front().unwrap();
					}
					self.clients.iter_mut().for_each(|OtServerClient { offset, .. }| *offset -= x);
				}
			}
			true
		} else {
			false
		}
	}
	pub fn drained(&self) -> bool {
		self.pending.is_empty()
	}
}
impl<T> OtServerClient<T>
where
	T: State,
{
	fn new(to_client: Sender<Option<T::Instr>>, from_client: Receiver<Option<T::Instr>>) -> Self {
		Self { offset: 0, to_client, from_client }
	}
}
