use crate::driver::io::{Io, IoInner};
use alloc::vec::Vec;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use yage_util::list::{Link, LinkedList};

pub(super) struct Registrations {
    num_pending_release: AtomicUsize,
}

impl Registrations {
    pub fn new() -> (Self, Synced) {
        let this = Registrations {
            num_pending_release: AtomicUsize::new(0),
        };

        let synced = Synced {
            shutdown: false,
            registrations: LinkedList::new(),
            pending_drop: Vec::with_capacity(16),
        };
        (this, synced)
    }
}

pub(super) struct Synced {
    shutdown: bool,
    registrations: LinkedList<Io>,
    pending_drop: Vec<Io>,
}
