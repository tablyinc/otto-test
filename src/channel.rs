//! Simple implementation of single-thread MPMC channels, handy for testing.

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
	let channel = Rc::new(RefCell::new(VecDeque::new()));
	(Sender(channel.clone()), Receiver(channel))
}

#[derive(Clone, Debug)]
pub struct Sender<T>(Rc<RefCell<VecDeque<T>>>);

#[derive(Clone, Debug)]
pub struct Receiver<T>(Rc<RefCell<VecDeque<T>>>);

impl<T> Sender<T> {
	pub fn len(&self) -> usize {
		self.0.borrow().len()
	}
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
	pub fn send(&self, val: T) {
		self.0.borrow_mut().push_back(val);
	}
}

impl<T> Receiver<T> {
	pub fn len(&self) -> usize {
		self.0.borrow().len()
	}
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
	pub fn try_receive(&self) -> Option<T> {
		self.0.borrow_mut().pop_front()
	}
}
