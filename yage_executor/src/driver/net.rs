use crate::driver::{registrations::Registrations, registrations::Synced};
use std::io;
use std::sync::Mutex;
use std::time::Duration;
use yage_net::{
    Token,
    event_loop::{EventLoop, Events, Registry},
    notifier::Notifier,
    waker::IoWaker,
};

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
        let waker = IoWaker::new(e_loop.registry(), TOKEN_WAKEUP)?;
        let registry = e_loop.registry().try_clone()?;

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
            waker,
        };

        Ok((driver, handle))
    }

    fn drive(&mut self, handle: &NetHandle, max_wait: Option<Duration>) {}
}
