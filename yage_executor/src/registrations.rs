
use std::sync::{Mutex, Arc, atomic::{AtomicUsize, Ordering}};
use yage_util::list::{Link, LinkedList};

pub(super) struct Registrations {
  num_pending_release: AtomicUsize
}

pub(super) struct Synced {
  shutdown: bool,
  registrations: LinkedList<Arc<Io>>
  pending_drop: SmallVec<Arc<Io>>
}
