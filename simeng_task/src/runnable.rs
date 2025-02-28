use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::Poll;
use core::{marker::PhantomData, task::Waker};

use crate::flags;
use crate::{Header, RawPoll, state::State};

pub struct Task<M = ()> {
    ptr: NonNull<()>,
    _marker: PhantomData<M>,
}

impl<M> Task<M> {
    pub fn run(self, cx: &mut core::task::Context<'_>) -> RawPoll {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header<M>;
        core::mem::forget(self);

        unsafe { ((*header).vtable.poll)(ptr, &raw mut *cx) }
    }

    pub fn waker(&self) -> Waker {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header<M>;
        unsafe {
            let raw = ((*header).vtable.clone_waker)(ptr as *const ());
            Waker::from_raw(raw)
        }
    }

    pub fn schedule(self) {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header<M>;
        core::mem::forget(self);

        unsafe { ((*header).vtable.schedule)(ptr) }
    }
}

impl<M> Task<M> {
    pub(crate) const unsafe fn from_ptr(ptr: NonNull<()>) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }
}

pub struct Handle<T, M = ()> {
    pub(crate) ptr: NonNull<()>,
    pub(crate) _marker_types: PhantomData<(T, M)>,
}

impl<T, M> Unpin for Handle<T, M> {}

impl<T, M> Handle<T, M> {
    fn __set_detached(&mut self) -> Option<Result<T, crate::Panic>> {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header<M>;

        unsafe {
            let mut output = None;
            let state = (*header).state.load(core::sync::atomic::Ordering::Relaxed);
            let exp = State {
                waker_reference_count: 1,
                task_reference_count: 1,
                flags: flags::SCHEDULED | flags::TASK_ALIVE,
            };

            let current = State {
                flags: flags::SCHEDULED,
                ..state
            };

            if let Err(mut state) = (*header).state.compare_exchange_weak(
                exp,
                current,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                loop {
                    if state.is_completed() && !state.is_closed() {
                        match (*header).state.compare_exchange_weak(
                            state,
                            state.set_flag(flags::CLOSED),
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                output = Some(
                                    (((*header).vtable.get_output)(ptr)
                                        as *mut Result<T, crate::Panic>)
                                        .read(),
                                );

                                state = state.set_flag(flags::CLOSED).set_flag(flags::TAKEN);
                            }
                            Err(s) => state = s,
                        }
                    } else {
                        let new = if state.task_reference_count == 1 || state.is_closed() {
                            state
                                .set_flag(flags::SCHEDULED)
                                .set_flag(flags::CLOSED)
                                .increment_reference_count()
                        } else {
                            state.clear_flag(flags::TASK_ALIVE)
                        };

                        match (*header).state.compare_exchange_weak(
                            state,
                            new,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                if state.task_reference_count == 0 {
                                    if !state.is_closed() {
                                        ((*header).vtable.schedule)(ptr);
                                    } else {
                                        ((*header).vtable.drop_task)(ptr)
                                    }
                                }
                                break;
                            }
                            Err(s) => state = s,
                        }
                    }
                }
            }
            output
        }
    }

    fn __set_canceled(&mut self) {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header<M>;

        unsafe {
            let mut state = (*header).state.load(Ordering::Acquire);

            loop {
                if state.is_completed() || state.is_closed() {
                    break;
                }

                let new = if !state.is_running() && !state.is_scheduled() {
                    state
                        .set_flag(flags::CLOSED)
                        .set_flag(flags::SCHEDULED)
                        .increment_reference_count()
                } else {
                    state.set_flag(flags::CLOSED)
                };

                match (*header).state.compare_exchange_weak(
                    state,
                    new,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        if !state.is_scheduled() && !state.is_running() {
                            ((*header).vtable.schedule)(ptr);
                        }
                        break;
                    }
                    Err(s) => state = s,
                }
            }
        }
    }

    pub fn detach(self) {
        let mut this = self;
        let _out = this.__set_detached();
        core::mem::forget(this);
    }

    fn __poll_task(&mut self) -> Poll<Option<T>> {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header<M>;

        unsafe {
            let mut state = (*header).state.load(Ordering::Acquire);

            loop {
                if state.has_flag_set(flags::TAKEN) {
                    return Poll::Ready(None);
                }

                if state.is_closed() {
                    if state.is_scheduled() || state.is_running() {
                        return Poll::Pending;
                    }

                    return Poll::Ready(None);
                }

                if !state.is_completed() {
                    if state.is_closed() {
                        continue;
                    }

                    return Poll::Pending;
                }

                match (*header).state.compare_exchange_weak(
                    state,
                    state.set_flag(flags::CLOSED),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        let output =
                            ((*header).vtable.get_output)(ptr) as *mut Result<T, crate::Panic>;
                        let output = output.read();

                        let output = match output {
                            Ok(out) => out,
                            Err(panic) => {
                                #[cfg(feature = "std")]
                                std::panic::resume_unwind(panic);

                                #[cfg(not(feature = "std"))]
                                match panic {}
                            }
                        };

                        return Poll::Ready(Some(output));
                    }
                    Err(s) => state = s,
                }
            }
        }
    }
}

impl<T, M> Future for Handle<T, M> {
    type Output = T;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let out = core::task::ready!(self.__poll_task()).expect("future polled after completion");
        Poll::Ready(out)
    }
}

impl<T, M> Drop for Handle<T, M> {
    fn drop(&mut self) {
        self.__set_canceled();
        self.__set_detached();
    }
}
