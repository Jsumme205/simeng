#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod sealed {
    use crate::runnable;

    pub trait Sealed<M> {}

    impl<M, F> Sealed<M> for F where F: Fn(runnable::Task<M>) {}
}

pub trait Schedule<Meta = ()>: sealed::Sealed<Meta> {
    fn schedule(&self, runnable: runnable::Task<Meta>);
}

impl<Fun, Meta> Schedule<Meta> for Fun
where
    Fun: Fn(runnable::Task<Meta>),
{
    fn schedule(&self, runnable: runnable::Task<Meta>) {
        (self)(runnable)
    }
}

use core::future::Future;

use core::pin::Pin;
use core::ptr::NonNull;
use core::sync::atomic::Ordering;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

#[cfg(feature = "std")]
type Panic = alloc::boxed::Box<dyn std::any::Any + Send + 'static>;

#[cfg(not(feature = "std"))]
type Panic = core::convert::Infallible;

use state::AtomicState;

pub mod builder;
mod flags;
mod layout;
pub mod runnable;
mod state;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RawPoll {
    Pending,
    Complete,
}

pub struct TaskVtable {
    schedule: unsafe fn(*mut ()),
    wake: unsafe fn(*const ()),
    wake_by_ref: unsafe fn(*const ()),
    get_output: unsafe fn(*mut ()) -> *mut (),
    poll: unsafe fn(*mut (), *mut Context<'_>) -> RawPoll,
    drop_waker: unsafe fn(*const ()),
    drop_reference: unsafe fn(*mut ()),
    clone_waker: unsafe fn(*const ()) -> RawWaker,
    drop_task: unsafe fn(*mut ()),
    drop_future: unsafe fn(*mut ()),
    layout: &'static TaskLayout,
}

struct TaskLayout {
    layout: core::alloc::Layout,
    offset_s: usize,
    offset_task: usize,
}

struct Header<Metadata> {
    state: AtomicState,
    metadata: Metadata,
    vtable: &'static TaskVtable,
}

#[repr(C)]
struct RawTask<Metadata, Fut, Sched>
where
    Fut: Future,
{
    header: *const Header<Metadata>,
    schedule: *const Sched,
    future: TaskFuture<Fut>,
}

impl<M, F, S> Copy for RawTask<M, F, S> where F: Future {}

impl<M, F, S> Clone for RawTask<M, F, S>
where
    F: Future,
{
    fn clone(&self) -> Self {
        *self
    }
}

union TaskFuture<Fut>
where
    Fut: Future,
{
    future: *mut Fut,
    result: *mut Result<Fut::Output, Panic>,
}

impl<F> Copy for TaskFuture<F> where F: Future {}

impl<F> Clone for TaskFuture<F>
where
    F: Future,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Metadata, Fut, Sched> RawTask<Metadata, Fut, Sched>
where
    Fut: Future,
    Sched: Schedule<Metadata>,
{
    const fn eval_task_layout() -> TaskLayout {
        use self::layout::ConstLayout;

        let layout_header = ConstLayout::new_for::<Header<Metadata>>();
        let layout_schedule = ConstLayout::new_for::<Sched>();
        let layout_future = ConstLayout::new_for::<Fut>();
        let layout_result = ConstLayout::new_for::<Result<Fut::Output, Panic>>();
        // SAFETY: we come from valid sizes and alignments
        let union_layout = unsafe { ConstLayout::unionize(layout_future, layout_result) };

        let (layout, offset_s) = layout_header.extend(layout_schedule).unwrap();
        let (layout, offset_union) = layout.extend(union_layout).unwrap();
        TaskLayout {
            layout: unsafe { layout.into_standard_layout() },
            offset_s,
            offset_task: offset_union,
        }
    }

    const TASK_LAYOUT: TaskLayout = Self::eval_task_layout();

    const WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_waker,
        Self::wake,
        Self::wake_by_ref,
        Self::drop_waker,
    );

    fn allocate<'a, Gen>(future: Gen, schedule: Sched, metadata: Metadata) -> NonNull<()>
    where
        Gen: FnOnce(&'a Metadata) -> Fut,
        Fut: 'a,
        Metadata: 'a,
    {
        let task_layout = Self::TASK_LAYOUT;

        unsafe {
            let ptr = match NonNull::new(alloc::alloc::alloc(task_layout.layout) as *mut ()) {
                None => abort(),
                Some(ptr) => ptr,
            };

            let p = ptr.as_ptr();

            let raw = Self::from_ptr(p, false);

            (raw.header as *mut Header<Metadata>).write(Header {
                state: AtomicState::new(state::State {
                    waker_reference_count: 1,
                    task_reference_count: 1,
                    flags: flags::SCHEDULED | flags::TASK_ALIVE,
                }),
                metadata,
                vtable: &TaskVtable {
                    schedule: Self::schedule,
                    wake: Self::wake,
                    wake_by_ref: Self::wake_by_ref,
                    get_output: Self::get_output,
                    poll: Self::poll,
                    drop_waker: Self::drop_waker,
                    clone_waker: Self::clone_waker,
                    drop_task: Self::drop_task,
                    drop_reference: Self::drop_reference,
                    drop_future: Self::drop_future,
                    layout: &Self::TASK_LAYOUT,
                },
            });

            (raw.schedule as *mut Sched).write(schedule);
            (raw.future.future).write((future)(&(*raw.header).metadata));
            ptr
        }
    }

    /// SAFETY:
    /// 1. `ptr` must come from a previously allocated instance of `RawTask`
    /// 2. `has_completed` must only be `true` if
    unsafe fn from_ptr(ptr: *const (), has_completed: bool) -> Self {
        let task_layout = Self::TASK_LAYOUT;
        let ptr = ptr as *const u8;
        let header = ptr as *const Header<Metadata>;
        unsafe {
            if has_completed {
                Self {
                    header,
                    schedule: ptr.add(task_layout.offset_s) as *const Sched,
                    future: TaskFuture {
                        result: ptr.add(task_layout.offset_task) as *mut Result<Fut::Output, Panic>,
                    },
                }
            } else {
                Self {
                    header,
                    schedule: ptr.add(task_layout.offset_s) as *const Sched,
                    future: TaskFuture {
                        future: ptr.add(task_layout.offset_task) as *mut Fut,
                    },
                }
            }
        }
    }

    unsafe fn header(ptr: *const ()) -> *const Header<Metadata> {
        ptr as *const Header<Metadata>
    }

    unsafe fn has_completed(ptr: *const ()) -> bool {
        unsafe {
            (*Self::header(ptr)).state.with_acquire_release(|state| {
                state.has_flag_set(flags::COMPLETED | flags::CLOSED | flags::CANCELLED)
            })
        }
    }

    unsafe fn schedule(ptr: *mut ()) {
        let raw = unsafe {
            let has_completed = Self::has_completed(ptr);
            Self::from_ptr(ptr, has_completed)
        };

        let _waker;
        if core::mem::size_of::<Sched>() > 0 {
            _waker = unsafe { Waker::from_raw(Self::clone_waker(ptr)) };
        }

        let task = unsafe { runnable::Task::from_ptr(NonNull::new_unchecked(ptr)) };
        unsafe {
            (*raw.schedule).schedule(task);
        }
    }

    unsafe fn wake(ptr: *const ()) {
        use core::mem;

        if mem::size_of::<Sched>() > 0 {
            unsafe { Self::wake_by_ref(ptr) };
            unsafe { Self::drop_waker(ptr) };
            return;
        }

        unsafe {
            let raw = Self::from_ptr(ptr, Self::has_completed(ptr));
            let mut state = (*raw.header).state.load(Ordering::Acquire);
            loop {
                if state.has_flag_set(flags::COMPLETED | flags::CLOSED) {
                    Self::drop_waker(ptr);
                    break;
                }

                if state.has_flag_set(flags::SCHEDULED) {
                    match (*raw.header).state.compare_exchange_weak(
                        state,
                        state,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            Self::drop_waker(ptr);
                            break;
                        }
                        Err(s) => state = s,
                    }
                } else {
                    match (*raw.header).state.compare_exchange_weak(
                        state,
                        state.set_flag(flags::SCHEDULED),
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            if state.has_flag_set(flags::RUNNING) {
                                Self::schedule(ptr as *mut ());
                            } else {
                                Self::drop_waker(ptr);
                            }
                            break;
                        }
                        Err(s) => state = s,
                    }
                }
            }
        }
    }

    unsafe fn wake_by_ref(ptr: *const ()) {
        let raw = unsafe { Self::from_ptr(ptr, Self::has_completed(ptr)) };

        let mut state = unsafe { (*raw.header).state.load(Ordering::Acquire) };

        loop {
            if state.has_flag_set(flags::COMPLETED | flags::CLOSED | flags::CANCELLED) {
                break;
            }

            if state.has_flag_set(flags::SCHEDULED) {
                match unsafe {
                    (*raw.header).state.compare_exchange_weak(
                        state,
                        state,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                } {
                    Ok(_) => break,
                    Err(s) => state = s,
                }
            } else {
                let new = if !state.has_flag_set(flags::RUNNING) {
                    state.increment_reference_count().set_flag(flags::SCHEDULED)
                } else {
                    state.set_flag(flags::SCHEDULED)
                };

                match unsafe {
                    (*raw.header).state.compare_exchange_weak(
                        state,
                        new,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                } {
                    Ok(_) => {
                        if !state.has_flag_set(flags::RUNNING) {
                            if state.task_reference_count > i16::MAX as _ {
                                abort();
                            }

                            let task: runnable::Task<Metadata> = unsafe {
                                runnable::Task::from_ptr(NonNull::new_unchecked(ptr as *mut ()))
                            };
                            unsafe {
                                (*raw.schedule).schedule(task);
                            }
                        }
                        break;
                    }
                    Err(s) => state = s,
                }
            }
        }
    }

    unsafe fn get_output(ptr: *mut ()) -> *mut () {
        let raw = unsafe {
            let has_completed = Self::has_completed(ptr);
            Self::from_ptr(ptr, has_completed)
        };
        unsafe { raw.future.result as *mut () }
    }

    unsafe fn poll(ptr: *mut (), cx: *mut Context<'_>) -> RawPoll {
        let raw = unsafe {
            let has_completed = Self::has_completed(ptr);
            Self::from_ptr(ptr, has_completed)
        };

        let mut state = unsafe { (*raw.header).state.load(Ordering::Acquire) };
        loop {
            if state.has_flag_set(flags::CLOSED) {
                unsafe { Self::drop_future(ptr) };

                unsafe {
                    (*raw.header)
                        .state
                        .store(state.clear_flag(flags::SCHEDULED), Ordering::AcqRel);
                };

                unsafe {
                    Self::drop_reference(ptr);
                }

                return RawPoll::Complete;
            }

            match unsafe {
                (*raw.header).state.compare_exchange_weak(
                    state,
                    state.clear_flag(flags::SCHEDULED).set_flag(flags::RUNNING),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
            } {
                Ok(_) => {
                    state.clear_flag(flags::SCHEDULED).set_flag(flags::RUNNING);
                    break;
                }
                Err(s) => state = s,
            }
        }

        let _guard = Guard(raw);

        #[cfg(not(feature = "std"))]
        let poll: Poll<Result<<Fut as Future>::Output, Panic>> = unsafe {
            <Fut as Future>::poll(Pin::new_unchecked(&mut *raw.future.future), &mut *cx).map(Ok)
        };

        #[cfg(feature = "std")]
        let poll: Poll<Result<<Fut as Future>::Output>, Panic> = unsafe {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                <Fut as Future>::poll(Pin::new_unchecked(&mut *raw.future.future), &mut *cx)
            })) {
                Ok(Poll::Ready(out)) => Poll::Ready(Ok(out)),
                Ok(Poll::Pending) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        };

        core::mem::forget(_guard);

        match poll {
            Poll::Ready(out) => {
                unsafe { Self::drop_future(ptr) };
                unsafe { raw.future.result.write(out) };

                loop {
                    let new = if !state.has_flag_set(flags::TASK_ALIVE) {
                        state
                            .clear_flag(flags::RUNNING)
                            .clear_flag(flags::SCHEDULED)
                            .set_flag(flags::COMPLETED)
                            .set_flag(flags::CLOSED)
                    } else {
                        state
                            .clear_flag(flags::RUNNING)
                            .clear_flag(flags::SCHEDULED)
                            .set_flag(flags::COMPLETED)
                    };

                    match unsafe {
                        (*raw.header).state.compare_exchange_weak(
                            state,
                            new,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                    } {
                        Ok(_) => {
                            if !state.has_flag_set(flags::TASK_ALIVE)
                                || state.has_flag_set(flags::CLOSED)
                            {
                                abort_on_panic(|| unsafe { raw.future.result.drop_in_place() });

                                unsafe {
                                    Self::drop_reference(ptr);
                                }
                                return RawPoll::Complete;
                            }
                        }
                        Err(s) => state = s,
                    }
                }
            }
            Poll::Pending => {
                let mut future_dropped = false;

                loop {
                    let new = if state.has_flag_set(flags::CLOSED) {
                        state
                            .clear_flag(flags::RUNNING)
                            .clear_flag(flags::SCHEDULED)
                    } else {
                        state.clear_flag(flags::RUNNING)
                    };

                    if state.has_flag_set(flags::CLOSED) && !future_dropped {
                        unsafe {
                            Self::drop_future(ptr);
                            future_dropped = true;
                        }
                    }

                    match unsafe {
                        (*raw.header).state.compare_exchange_weak(
                            state,
                            new,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                    } {
                        Ok(state) => {
                            if state.has_flag_set(flags::CLOSED) {
                                unsafe {
                                    Self::drop_reference(ptr);
                                }
                            } else if state.has_flag_set(flags::SCHEDULED) {
                                unsafe {
                                    Self::schedule(ptr);
                                }
                                return RawPoll::Pending;
                            } else {
                                unsafe { Self::drop_reference(ptr) };
                                return RawPoll::Pending;
                            }
                        }
                        Err(s) => state = s,
                    }
                }
            }
        }

        struct Guard<F, S, M>(RawTask<M, F, S>)
        where
            F: Future,
            S: Schedule<M>;

        impl<F, S, M> Drop for Guard<F, S, M>
        where
            F: Future,
            S: Schedule<M>,
        {
            fn drop(&mut self) {
                let raw = self.0;
                let ptr = raw.header as *mut ();

                unsafe {
                    let mut state = (*raw.header).state.load(Ordering::Acquire);

                    loop {
                        if state.has_flag_set(flags::SCHEDULED) {
                            RawTask::<M, F, S>::drop_future(ptr);
                            (*raw.header)
                                .state
                                .with(Ordering::AcqRel, Ordering::AcqRel, |state| {
                                    *state = state
                                        .clear_flag(flags::RUNNING)
                                        .clear_flag(flags::SCHEDULED);
                                });

                            RawTask::<M, F, S>::drop_reference(ptr);
                            break;
                        }

                        match (*raw.header).state.compare_exchange_weak(
                            state,
                            state
                                .clear_flag(flags::RUNNING)
                                .clear_flag(flags::SCHEDULED)
                                .set_flag(flags::CLOSED),
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                RawTask::<M, F, S>::drop_future(ptr);
                                RawTask::<M, F, S>::drop_reference(ptr);
                                break;
                            }
                            Err(s) => state = s,
                        }
                    }
                }
            }
        }
    }

    unsafe fn drop_waker(ptr: *const ()) {
        unsafe {
            let header = Self::header(ptr);

            (*header).state.with_acquire_release(|state| {
                *state = state.decrement_waker_count();
                (*header).state.store(*state, Ordering::AcqRel);
                if state.task_reference_count == 1 && !state.has_flag_set(flags::TASK_ALIVE) {
                    if !state.has_flag_set(flags::COMPLETED | flags::CLOSED) {
                        (*header).state.store(
                            state.set_flag(flags::SCHEDULED | flags::CLOSED),
                            Ordering::Release,
                        );
                        Self::schedule(ptr as *mut ());
                    } else {
                        Self::drop_task(ptr as *mut ());
                    }
                }
            });
        }
    }

    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        let header = unsafe { Self::header(ptr) };
        unsafe {
            (*header)
                .state
                .with(Ordering::Relaxed, Ordering::Relaxed, |state| {
                    *state = state.increment_reference_count();
                });
            RawWaker::new(ptr, &Self::WAKER_VTABLE)
        }
    }

    unsafe fn drop_reference(ptr: *mut ()) {
        unsafe {
            let (raw, has_task) = (*Self::header(ptr)).state.with_acquire_release(|state| {
                *state = state.decrement_reference_count();
                (
                    state.task_reference_count,
                    state.has_flag_set(flags::TASK_ALIVE),
                )
            });

            if raw == 0 && !has_task {
                Self::drop_task(ptr);
            }
        }
    }

    unsafe fn drop_task(ptr: *mut ()) {
        let raw = unsafe {
            let has_completed = Self::has_completed(ptr);
            Self::from_ptr(ptr, has_completed)
        };

        let task_layout = Self::TASK_LAYOUT;

        abort_on_panic(|| unsafe {
            (raw.header as *mut Header<Metadata>).drop_in_place();
            (raw.schedule as *mut Sched).drop_in_place();
        });

        unsafe {
            alloc::alloc::dealloc(ptr as *mut u8, task_layout.layout);
        }
    }

    unsafe fn drop_future(ptr: *mut ()) {
        let raw = unsafe {
            let h_c = Self::has_completed(ptr);
            Self::from_ptr(ptr, h_c)
        };
        abort_on_panic(|| unsafe {
            raw.future.future.drop_in_place();
        });
    }
}

const fn max(lhs: usize, rhs: usize) -> usize {
    if lhs > rhs { lhs } else { rhs }
}

pub(crate) fn abort() -> ! {
    let _panic = RunOnDrop::new(|| panic!("aborting"));
    panic!("aborting")
}

struct RunOnDrop<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> RunOnDrop<F> {
    fn new(f: F) -> Self {
        Self(Some(f))
    }
}

impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        let f = self
            .0
            .take()
            .expect("this really shouldn't happen (double drop)");
        f();
    }
}

fn abort_on_panic<T>(f: impl FnOnce() -> T) -> T {
    let _bomp = RunOnDrop::new(|| abort());
    let t = f();
    core::mem::forget(_bomp);
    t
}
