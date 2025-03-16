pub mod event_loop;
pub mod notifier;
mod unix;
pub mod waker;

use core::num::NonZero;

pub struct Token(pub usize);

pub struct Interest(NonZero<u8>);

impl Interest {
    pub const READABLE: Self = unsafe { Self(NonZero::new_unchecked(0b0001)) };
}
