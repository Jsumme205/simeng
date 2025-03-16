use core::{cell::UnsafeCell, marker::PhantomData};

mod borrow;

use borrow::BorrowFlag;

pub struct Atomic<T: ?Sized> {
    borrow: BorrowFlag,
    value: UnsafeCell<T>,
}

pub struct AtomicRef<'a, T: ?Sized> {
    _marker: PhantomData<&'a T>,
}

pub struct AtomicMut<'a, T: ?Sized> {
    _marker: PhantomData<&'a mut T>,
}

impl<T> Atomic<T> {
    pub const fn new(val: T) -> Self {
        Self {
            borrow: borrow::new_borrow_flag(),
            value: UnsafeCell::new(val),
        }
    }
}

impl<T: ?Sized> Atomic<T> {
    pub fn borrow_mut(&self) -> AtomicMut<'_, T> {
        todo!()
    }
}
