#![cfg_attr(not(feature = "std"), no_std)]

use core::task::RawWaker;

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

pub mod task;

mod flags;
mod header;
mod layout;
mod state;

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

use header::Header;
use layout::ConstLayout;

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
    drop_reference: unsafe fn(*const (), ReferenceKind),
    destroy: unsafe fn(*mut ()),
    run: unsafe fn(*mut ()) -> bool,
    clone_waker: unsafe fn(*const ()) -> RawWaker,
    task_layout: &'static TaskLayout,
}

struct Raw<F, T, S, M> {
    header: *const Header<M>,
    schedule: *const S,
    future: *mut F,
    result: *mut Result<T, Panic>,
}

impl<F, T, S, M> Copy for Raw<F, T, S, M> {}

impl<F, T, S, M> Clone for Raw<F, T, S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, T, S, M> Raw<F, T, S, M> {
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
