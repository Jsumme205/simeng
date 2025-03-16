use crate::{Interest, Token, notifier::Notifier, unix::selector::Selector};
use std::{
    env::Vars,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub struct Registry {
    pub(crate) selector: Selector,
    has_waker: Arc<AtomicBool>,
}

pub struct EventLoop {
    registry: Registry,
}

impl EventLoop {
    pub fn new() -> io::Result<Self> {
        Selector::new().map(|sel| Self {
            registry: Registry {
                selector: sel,
                has_waker: Arc::new(AtomicBool::new(false)),
            },
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn poll(
        &mut self,
        events: &mut Vec<libc::epoll_event>,
        timeout: Option<Duration>,
    ) -> io::Result<()> {
        self.registry.selector.select(events, timeout)
    }
}

impl Registry {
    pub fn register<N>(&self, notifier: &mut N, token: Token, interests: Interest) -> io::Result<()>
    where
        N: Notifier + ?Sized,
    {
        notifier.register(self, token, interests)
    }

    pub fn reregister<N>(
        &self,
        notifier: &mut N,
        token: Token,
        interests: Interest,
    ) -> io::Result<()>
    where
        N: Notifier + ?Sized,
    {
        notifier.reregister(self, token, interests)
    }

    pub fn deregister<N>(&self, notifier: &mut N) -> io::Result<()>
    where
        N: Notifier + ?Sized,
    {
        notifier.deregister(self)
    }

    pub(crate) fn register_waker(&self) {
        assert!(!self.has_waker.swap(true, Ordering::AcqRel), "no.")
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        self.selector.try_clone().map(|this| Self {
            selector: this,
            has_waker: Arc::clone(&self.has_waker),
        })
    }
}

pub type Events = Vec<libc::epoll_event>;
