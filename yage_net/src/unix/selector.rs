use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use libc::{EPOLLET, EPOLLIN, EPOLLOUT, EPOLLPRI, EPOLLRDHUP};

fn next_id() -> usize {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug)]
pub struct Selector {
    id: usize,
    fd: OwnedFd,
}

impl Selector {
    pub fn new() -> io::Result<Self> {
        let ep = unsafe {
            super::wrap_error(|| {
                let fd = libc::epoll_create1(libc::EPOLL_CLOEXEC);
                (fd, OwnedFd::from_raw_fd(fd))
            })?
        };
        Ok(Self {
            id: next_id(),
            fd: ep,
        })
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        self.fd.try_clone().map(|ep| Self {
            id: self.id,
            fd: ep,
        })
    }

    pub fn select(
        &self,
        events: &mut Vec<libc::epoll_event>,
        timeout: Option<Duration>,
    ) -> io::Result<()> {
        let to = timeout
            .map(|to| {
                to.checked_add(Duration::from_nanos(999_999))
                    .unwrap_or(to)
                    .as_millis() as libc::c_int
            })
            .unwrap_or(-1);

        events.clear();

        super::wrap_error(|| unsafe {
            let ret = libc::epoll_wait(
                self.fd.as_raw_fd(),
                events.as_mut_ptr(),
                events.capacity() as _,
                to,
            );
            (ret, ret)
        })
        .map(|n_events| unsafe {
            events.set_len(n_events as _);
        })
    }

    pub fn register(
        &self,
        fd: RawFd,
        token: crate::Token,
        interests: crate::Interest,
    ) -> io::Result<()> {
        let mut event = libc::epoll_event {
            events: interest_to_epoll(interests),
            u64: token.0 as _,
            #[cfg(target_os = "redox")]
            _pad: 0,
        };

        let ep = self.fd.as_raw_fd();
        super::wrap_error(|| unsafe {
            (libc::epoll_ctl(ep, libc::EPOLL_CTL_ADD, fd, &mut event), ())
        })
    }

    pub fn reregister(
        &self,
        fd: RawFd,
        token: crate::Token,
        interests: crate::Interest,
    ) -> io::Result<()> {
        let mut event = libc::epoll_event {
            events: interest_to_epoll(interests),
            u64: token.0 as _,
            #[cfg(target_os = "redox")]
            _pad: 0,
        };

        let ep = self.fd.as_raw_fd();
        super::wrap_error(|| unsafe {
            (
                libc::epoll_ctl(ep, libc::EPOLL_CTL_MOD, fd, &raw mut event),
                (),
            )
        })
    }

    pub fn deregister(&self, fd: RawFd) -> io::Result<()> {
        let ep = self.fd.as_raw_fd();
        super::wrap_error(|| unsafe {
            (
                libc::epoll_ctl(ep, libc::EPOLL_CTL_DEL, fd, core::ptr::null_mut()),
                (),
            )
        })
    }
}

fn interest_to_epoll(interests: crate::Interest) -> u32 {
    todo!()
}
