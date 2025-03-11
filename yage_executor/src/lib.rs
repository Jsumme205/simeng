#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std as alloc;

use alloc::sync::Arc;
use concurrent_queue::ConcurrentQueue;
use core::{
    marker::PhantomData,
    sync::atomic::{AtomicPtr, AtomicUsize},
    task::Waker,
};
use simeng_task::{builder::Builder, runnable::Task};
use slab::Slab;

/// TODO: when stablized, change this back to private
pub mod handle;

#[cfg(feature = "std")]
mod net;

#[cfg(feature = "std")]
mod registrations;

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::ThreadSafe {}
    impl Sealed for super::NotThreadSafe {}
}

pub trait ExecutorMarker: sealed::Sealed {
    type Marker;
}

pub enum ThreadSafe {}

impl ExecutorMarker for ThreadSafe {
    type Marker = ();
}

pub enum NotThreadSafe {}

impl ExecutorMarker for NotThreadSafe {
    type Marker = PhantomData<*mut ()>;
}

struct ExecutorInner {
    task_queue: ConcurrentQueue<Task<usize>>,
    wakers: Arc<AtomicPtr<Slab<Waker>>>,
}

pub struct Executor<M: ExecutorMarker> {
    inner: ExecutorInner,
    task_id: AtomicUsize,
    #[cfg(feature = "std")]
    reactor_handle: Option<std::thread::JoinHandle<()>>,
    _marker: PhantomData<M>,
}

unsafe impl<M: ExecutorMarker> Send for Executor<M> where M::Marker: Send + Sync {}
unsafe impl<M: ExecutorMarker> Sync for Executor<M> where M::Marker: Send + Sync {}

impl Executor<ThreadSafe> {
    pub fn new_sync() -> Self {
        Self {
            inner: ExecutorInner {
                task_queue: ConcurrentQueue::unbounded(),
                wakers: Arc::new(AtomicPtr::new(core::ptr::null_mut())),
            },
            task_id: AtomicUsize::new(1),
            #[cfg(feature = "std")]
            reactor_handle: None,
            _marker: PhantomData,
        }
    }
}

impl Executor<NotThreadSafe> {
    pub fn new_unsync() -> Self {
        Self {
            inner: ExecutorInner {
                task_queue: ConcurrentQueue::unbounded(),
                wakers: Arc::new(AtomicPtr::new(core::ptr::null_mut())),
            },
            task_id: AtomicUsize::new(0),
            #[cfg(feature = "std")]
            reactor_handle: None,
            _marker: PhantomData,
        }
    }
}

pub struct TaskHandle<T>(handle::TaskHandle<'static, T, usize>);

impl<T> TaskHandle<T> {
    pub fn join(self) -> Option<T> {
        self.0.join()
    }
}
