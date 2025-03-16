use std::io;

use crate::{Token, event_loop::Registry, unix};

pub struct IoWaker {
    inner: unix::waker::Waker,
}

impl IoWaker {
    pub fn new(registry: &Registry, token: Token) -> io::Result<Self> {
        unix::waker::Waker::new(&registry.selector, token).map(|wk| Self { inner: wk })
    }

    pub fn wake(&self) -> io::Result<()> {
        self.inner.wake()
    }
}
