#![no_std]

/// TODOS:
/// 1. finish api for `ConcurrentQueue`
/// 2. work on `ArcConcurrentQueue`
use core::{marker::PhantomData, ptr::NonNull, sync::atomic::AtomicUsize};

extern crate alloc;

mod bounded;
mod unbounded;

#[repr(align(128))]
struct CachePadded<T>(pub T);

impl<T> core::ops::Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> core::ops::DerefMut for CachePadded<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub enum PushError<T> {
    Closed(T),
    Full(T),
}

pub enum PopError {
    Closed,
    Empty,
}

pub(crate) trait UnsafeCellExt {
    type Value;

    fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(*mut Self::Value) -> R;
}

pub(crate) trait AtomicExt {
    type Value;

    fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self::Value) -> R;
}

impl<T> AtomicExt for core::sync::atomic::AtomicPtr<T> {
    type Value = *mut T;

    fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self::Value) -> R,
    {
        f(self.get_mut())
    }
}

impl AtomicExt for core::sync::atomic::AtomicUsize {
    type Value = usize;

    fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self::Value) -> R,
    {
        f(self.get_mut())
    }
}

impl<T> UnsafeCellExt for core::cell::UnsafeCell<T> {
    type Value = T;

    fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(*mut Self::Value) -> R,
    {
        f(self.get())
    }
}

fn full_fence() {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        use core::{arch::asm, cell::UnsafeCell};
        // HACK(stjepang): On x86 architectures there are two different ways of executing
        // a `SeqCst` fence.
        //
        // 1. `atomic::fence(SeqCst)`, which compiles into a `mfence` instruction.
        // 2. A `lock <op>` instruction.
        //
        // Both instructions have the effect of a full barrier, but empirical benchmarks have shown
        // that the second one is sometimes a bit faster.
        let a = UnsafeCell::new(0_usize);
        // It is common to use `lock or` here, but when using a local variable, `lock not`, which
        // does not change the flag, should be slightly more efficient.
        // Refs: https://www.felixcloutier.com/x86/not
        unsafe {
            #[cfg(target_pointer_width = "64")]
            asm!("lock not qword ptr [{0}]", in(reg) a.get(), options(nostack, preserves_flags));
            #[cfg(target_pointer_width = "32")]
            asm!("lock not dword ptr [{0:e}]", in(reg) a.get(), options(nostack, preserves_flags));
        }
        return;
    }
    #[allow(unreachable_code)]
    {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    }
}

pub struct ArcConcurrentQueue<T> {
    ptr: NonNull<Packet<T>>,
    _marker: PhantomData<Packet<T>>,
}

pub struct ConcurrentQueue<T> {
    inner: QueueFlavor<T>,
}

impl<T> ConcurrentQueue<T> {
    pub const fn unbounded() -> Self {
        Self {
            inner: QueueFlavor::Unbounded(unbounded::Unbounded::new()),
        }
    }

    pub fn push(&self, value: T) -> Result<(), PushError<T>> {
        match &self.inner {
            QueueFlavor::Unbounded(ub) => ub.push(value),
        }
    }

    pub fn pop(&self) -> Result<T, PopError> {
        match &self.inner {
            QueueFlavor::Unbounded(ub) => ub.pop(),
        }
    }
}

enum QueueFlavor<T> {
    Unbounded(unbounded::Unbounded<T>),
}

struct Packet<T> {
    reference_count: AtomicUsize,
    flavor: QueueFlavor<T>,
}
