pub mod selector;
pub mod waker;

use std::io;

pub(crate) fn wrap_error<R>(f: impl FnOnce() -> (i32, R)) -> io::Result<R> {
    let (r, ret) = f();
    if r < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}
