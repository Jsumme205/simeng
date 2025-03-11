use yage_util::{list::{Link, LinkedList, Pointers}, atomic::Atomic};
use yage_net::Interest;
use std::cell::UnsafeCell;
use std::marker::PhantomPinned;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Waker, Context};
use std::ptr::NonNull;

pub(super) struct Io {
    pointers: UnsafeCell<Pointers<Self>>,
    readiness: AtomicUsize,
    waiters: Atomic<Waiters>
}

struct Waiters {
  list: LinkedList<Waiter>
  reader: Option<Waker>,
  writer: Option<Waker>
}

struct Waiter {
  pointers: Pointers<Self>,
  waker: Option<Waker>,
  interests: Interest,
  ready: bool,
  _pin: PhantomPinned,
}


impl Waiter {
  unsafe fn address_of_pointers(this: NonNull<Self>) -> NonNull<Pointers<Self> {
    unsafe {
      NonNull::new_unchecked(&raw mut (*this.as_ptr()).pointers)
    }
  }
}

