use alloc::sync::Arc;
use std::cell::UnsafeCell;
use std::marker::PhantomPinned;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Waker};
use yage_net::Interest;
use yage_util::{
    atomic::Atomic,
    list::{Link, LinkedList, Pointers},
};

pub(super) struct IoInner {
    pointers: UnsafeCell<Pointers<Self>>,
    readiness: AtomicUsize,
    waiters: Atomic<Waiters>,
}

pub(super) struct Io(Arc<IoInner>);

unsafe impl Link for Io {
    type Handle = Arc<IoInner>;
    type Target = IoInner;

    fn as_raw(handle: &Self::Handle) -> NonNull<Self::Target> {
        unsafe { NonNull::new_unchecked(Arc::as_ptr(handle) as *mut _) }
    }

    unsafe fn from_raw(ptr: NonNull<Self::Target>) -> Self::Handle {
        unsafe { Arc::from_raw(ptr.as_ptr() as *const _) }
    }

    unsafe fn pointers(target: NonNull<Self::Target>) -> NonNull<Pointers<Self::Target>> {
        unsafe { NonNull::new_unchecked(target.as_ref().pointers.get()) }
    }
}

struct Waiters {
    list: LinkedList<Waiter>,
    reader: Option<Waker>,
    writer: Option<Waker>,
}

struct Waiter {
    pointers: Pointers<Self>,
    waker: Option<Waker>,
    interests: Interest,
    ready: bool,
    _pin: PhantomPinned,
}

impl Waiter {
    unsafe fn address_of_pointers(this: NonNull<Self>) -> NonNull<Pointers<Self>> {
        unsafe { NonNull::new_unchecked(&raw mut (*this.as_ptr()).pointers) }
    }
}

unsafe impl Link for Waiter {
    type Handle = NonNull<Waiter>;
    type Target = Waiter;

    fn as_raw(handle: &Self::Handle) -> NonNull<Self::Target> {
        *handle
    }

    unsafe fn from_raw(ptr: NonNull<Self::Target>) -> Self::Handle {
        ptr
    }

    unsafe fn pointers(target: NonNull<Self::Target>) -> NonNull<Pointers<Self::Target>> {
        unsafe { Waiter::address_of_pointers(target) }
    }
}
