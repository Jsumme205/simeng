#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;
use core::mem;
use core::sync::atomic::Ordering;
use core::task::{RawWaker, Waker};
use core::{future::Future, ptr::NonNull};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std as alloc;

macro_rules! std_println {
    ($($arg:tt)*) => {
        #[cfg(feature = "std")]
        std::println!($($arg)*)
    };
}

const fn max(lhs: usize, rhs: usize) -> usize {
    if lhs > rhs { lhs } else { rhs }
}

pub mod builder;
pub mod task;

mod flags;
mod header;
mod layout;
mod state;
mod utils;

mod sealed {
    use crate::task::Task;

    pub trait Sealed<M> {}

    impl<M, F> Sealed<M> for F where F: Fn(Task<M>) {}
}

pub trait Schedule<Meta = ()>: sealed::Sealed<Meta> {
    fn schedule(&self, runnable: task::Task<Meta>);
}

impl<Fun, Meta> Schedule<Meta> for Fun
where
    Fun: Fn(task::Task<Meta>),
{
    fn schedule(&self, runnable: task::Task<Meta>) {
        (self)(runnable)
    }
}

use header::{Header, Tag};
use layout::ConstLayout;
use task::Task;

#[cfg(feature = "std")]
type Panic = alloc::boxed::Box<dyn std::any::Any + Send + 'static>;

#[cfg(not(feature = "std"))]
type Panic = core::convert::Infallible;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ReferenceKind {
    Waker,
    Task,
}

struct TaskLayout {
    layout: core::alloc::Layout,
    offset_s: usize,
    offset_f: usize,
    offset_r: usize,
}

pub(crate) struct TaskVTable {
    schedule: unsafe fn(*const ()),
    drop_future: unsafe fn(*const ()),
    get_output: unsafe fn(*mut ()) -> *mut (),
    drop_reference: unsafe fn(*const ()),
    destroy: unsafe fn(*mut ()),
    run: unsafe fn(*mut ()) -> bool,
    clone_waker: unsafe fn(*const ()) -> RawWaker,
    task_layout: &'static TaskLayout,
}

struct Raw<F, T, S, M, E: Tag = ()> {
    header: *const Header<M, E>,
    schedule: *const S,
    future: *mut F,
    result: *mut Result<T, Panic>,
}

impl<F, T, S, M, E: Tag> Copy for Raw<F, T, S, M, E> {}

impl<F, T, S, M, E: Tag> Clone for Raw<F, T, S, M, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, T, S, M, E: Tag> Raw<F, T, S, M, E> {
    const fn eval_task_layout() -> TaskLayout {
        let layout_header = ConstLayout::new_for::<Header<M>>();
        let layout_s = ConstLayout::new_for::<S>();
        let layout_f = ConstLayout::new_for::<F>();
        let layout_r = ConstLayout::new_for::<Result<T, Panic>>();

        let size_union = max(layout_f.size(), layout_r.size());
        let align_union = max(layout_f.align(), layout_r.align());
        // SAFETY: these came from valid alignments
        let layout_union =
            unsafe { ConstLayout::from_size_align_unchecked(size_union, align_union) };

        let (layout, offset_s) = layout_header.extend(layout_s).unwrap();
        let (layout, offset_union) = layout.extend(layout_union).unwrap();
        TaskLayout {
            layout: unsafe { layout.into_standard_layout() },
            offset_s,
            offset_f: offset_union,
            offset_r: offset_union,
        }
    }
}

impl<F, T, S, M, E> Raw<F, T, S, M, E>
where
    F: Future<Output = T>,
    S: Schedule<M>,
    E: Tag,
{
    const TASK_LAYOUT: TaskLayout = Self::eval_task_layout();

    fn allocate<'a, Gen>(future: Gen, schedule: S, builder: builder::Builder<M, E>) -> NonNull<()>
    where
        Gen: FnOnce(&'a M) -> F,
        F: 'a,
        M: 'a,
    {
        let task_layout = Self::TASK_LAYOUT;

        unsafe {
            let ptr = match NonNull::new(alloc::alloc::alloc(task_layout.layout) as *mut ()) {
                None => utils::abort(),
                Some(ptr) => ptr,
            };

            let builder::Builder { metadata, tag } = builder;
            Header::new_in_place(
                metadata,
                &TaskVTable {
                    schedule: Self::schedule,
                    drop_future: Self::drop_future,
                    get_output: Self::get_output,
                    drop_reference: Self::drop_reference,
                    destroy: Self::destroy,
                    run: Self::run,
                    clone_waker: Self::clone_waker,
                    task_layout: &Self::TASK_LAYOUT,
                },
                tag,
                ptr.as_ptr(),
            );

            let raw = Self::from_ptr(ptr.as_ptr());

            (raw.schedule as *mut S).write(schedule);

            let future = utils::abort_on_panic(|| future(&(*raw.header).metadata));

            raw.future.write(future);

            ptr
        }
    }

    fn from_ptr(ptr: *const ()) -> Self {
        let task_layout = Self::TASK_LAYOUT;
        let p = ptr as *const u8;
        unsafe {
            Self {
                header: p as *const Header<M, E>,
                schedule: p.add(task_layout.offset_s).cast(),
                future: p.add(task_layout.offset_f) as *mut F,
                result: p.add(task_layout.offset_r) as *mut _,
            }
        }
    }

    unsafe fn schedule(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        let _waker;
        if mem::size_of::<S>() > 0 {
            _waker = unsafe { Waker::from_raw(Self::clone_waker(ptr)) }
        }

        let task = Task {
            ptr: unsafe { NonNull::new_unchecked(ptr as *mut _) },
            _marker: PhantomData::<M>,
        };
        unsafe {
            (*raw.schedule).schedule(task);
        }
    }

    unsafe fn wake(ptr: *const ()) {
        if core::mem::size_of::<S>() > 0 {
            unsafe {
                Self::wake_by_ref(ptr);
                Self::drop_waker(ptr);
            }
            return;
        }

        let raw = Self::from_ptr(ptr);

        let mut state = unsafe { (*raw.header).state.load(Ordering::Acquire) };

        loop {
            if state.is_completed() || state.is_closed() {
                unsafe { Self::drop_waker(ptr) };
                return;
            }

            if state.is_scheduled() {
                match unsafe {
                    (*raw.header).state.compare_exchange_weak(
                        state,
                        state,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                } {
                    Ok(_) => {}
                    Err(s) => state = s,
                }
            }
        }
    }

    unsafe fn wake_by_ref(ptr: *const ()) {}

    unsafe fn drop_waker(ptr: *const ()) {}

    unsafe fn drop_future(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        utils::abort_on_panic(|| unsafe {
            raw.future.drop_in_place();
        })
    }

    unsafe fn get_output(ptr: *mut ()) -> *mut () {
        let raw = Self::from_ptr(ptr);
        raw.future as *mut ()
    }

    unsafe fn drop_reference(ptr: *const ()) {}

    unsafe fn destroy(ptr: *mut ()) {}

    unsafe fn run(ptr: *mut ()) -> bool {
        false
    }

    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        todo!()
    }
}
