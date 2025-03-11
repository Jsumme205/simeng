
use yage_net::{event_loop::{Registry, EventLoop, Events}, notifier::Notifier, waker::IoWaker, Token};
use std::sync::Mutex;
use std::io;
use std::time::Duration;
use crate::{Registrations, Synced};

const TOKEN_WAKEUP: Token = Token(0);
const TOKEN_SIGNAL: Token = Token(1);

pub(crate) struct NetHandle {
  registry: Registry,
  registrations: Registrations,
  synced: Mutex<Synced>,
  waker: IoWaker,
}

pub(crate) struct NetDriver {
  signal_ready: bool,
  events: Events,
  event_loop: EventLoop, 
}

impl NetDriver {

  pub(crate) fn new(nevents: usize) -> io::Result<(NetDriver, NetHandle)> {
    let e_loop = EventLoop::new()?;
    let waker = IoWaker::new(poll.registry(), TOKEN_WAKEUP)?;
    let registry = poll.registry().try_clone()?;

    let driver = NetDriver {
      signal_ready: false,
      events: Events::with_capacity(nevents),
      event_loop: e_loop,
    };

    let (registrations, synced) = Registrations::new();

    let handle = NetHandle {
      registry,
      registrations,
      synced: Mutex::new(synced),
      waker
    };

    Ok((driver, handle))
  }

  fn drive(&mut self, handle: &Handle, max_wait: Option<Duration>) {
      handle.release_pending_registrations();
      
  }
    
  
}
