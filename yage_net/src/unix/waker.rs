use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};

use crate::{Interest, Token};

use super::selector::Selector;

pub(crate) struct Waker {
    inner: File,
}

impl Waker {
    pub(crate) fn new_unregistered() -> io::Result<Self> {
        let flags = libc::EFD_CLOEXEC | libc::EFD_NONBLOCK;
        let fd = super::wrap_error(|| unsafe {
            let r = libc::eventfd(0, flags);
            (r, r)
        })?;
        let file = unsafe { File::from_raw_fd(fd) };
        Ok(Self { inner: file })
    }

    pub(crate) fn new(selector: &Selector, token: Token) -> io::Result<Self> {
        let this = Self::new_unregistered()?;
        selector.register(this.inner.as_raw_fd(), token, Interest::READABLE)?;
        Ok(this)
    }

    pub(crate) fn wake(&self) -> io::Result<()> {
        #[cfg(target_os = "illumos")]
        self.reset()?;

        let buf: [u8; 8] = 1u64.to_ne_bytes();
        match (&self.inner).write(&buf) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                self.reset()?;
                self.wake()
            }
            Err(err) => Err(err),
        }
    }

    fn reset(&self) -> io::Result<()> {
        let mut buf: [u8; 8] = 0u64.to_ne_bytes();
        match (&self.inner).read(&mut buf) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(()),
            Err(e) => Err(e),
        }
    }
}
