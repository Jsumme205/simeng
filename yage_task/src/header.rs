use crate::state::{AtomicState, State};
use crate::{TaskVTable, flags};

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::task::Waker;

/// the header of a `Task`
///
/// this carries various bookkeeping fields, like the state and the actual vtable
/// this also carries 2 different fields for metadata, including a 16-bit `Tag`, as well as a `Metadata` field
pub(crate) struct Header<Metadata, T: Tag = ()> {
    pub(crate) state: AtomicState,
    pub(crate) awaiter: UnsafeCell<Option<Waker>>,
    pub(crate) vtable: &'static TaskVTable,
    pub(crate) metadata: Metadata,
    pub(crate) _marker: PhantomData<T>,
}

impl<Meta, T> Header<Meta, T>
where
    T: Tag,
{
    /// SAFETY: this must only be called when creating the header
    pub(crate) unsafe fn new_in_place(
        metadata: Meta,
        vtable: &'static TaskVTable,
        tag: T,
        ptr: *const (),
    ) {
        let ptr = ptr as *const Header<Meta, T> as *mut Header<Meta, T>;
        unsafe {
            ptr.write(Header {
                state: AtomicState::new(State {
                    reference_count: 1,
                    flags: flags::SCHEDULED | flags::HANDLE_HERE,
                    tag: tag.into_u16(),
                }),
                awaiter: UnsafeCell::new(None),
                vtable,
                metadata,
                _marker: PhantomData,
            });
        }
    }
}

/// the main trait for "tagging" a task.
/// this can be used to atomically indicate certain states or extra metadata
/// the only requirement is that it can fit into a `u16`
/// primitives with less than a 16-bit size implement this trait.
pub trait Tag {
    fn from_u16(val: u16) -> Self;

    fn into_u16(self) -> u16;
}

macro_rules! impl_tag_for_prims {
    ($($prim:ty)*) => {
       $(
        impl Tag for $prim {
            fn from_u16(val: u16) -> Self {
                val as $prim
            }

            fn into_u16(self) -> u16 {
                self as u16
            }
        }
       )*
    };
}

impl_tag_for_prims!(u8 u16 i8 i16);

impl Tag for () {
    fn from_u16(_: u16) -> Self {
        ()
    }

    fn into_u16(self) -> u16 {
        0
    }
}
