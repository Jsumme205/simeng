#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::alloc as allocator;

#[cfg(feature = "std")]
use std::alloc as allocator;

#[cfg(feature = "std")]
pub mod io_extension;

use core::{alloc::Layout, ptr::NonNull};

/// A 16-byte, growable vector similar to std's `Vec` type
/// for this to work, we assume 3 things
///
/// 1. The user doesn't need to allocate more than `u32::MAX` elements
///     for reference, this is 4_294_967_295 elements, which is quite generous
/// 2. The user doesn't need to customize allocators
/// 3. The user is dependant on structures to be small (E.G, you're `Box`ing a structure that needs to use a vector internally)
pub struct SmallVec<T> {
    ptr: NonNull<T>,
    len: u32,
    cap: u32,
}

impl<T> SmallVec<T> {
    /// creates a new `SmallVec` that hasn't heap-allocated any space
    ///
    /// this is good for const contexts
    ///
    /// see `std::vec::Vec::new()` for more info
    pub const fn new() -> Self {
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        assert!(
            capacity <= (u32::MAX - 1) as _,
            "capacity cannot be more than 2^32 - 1"
        );
        let ptr = unsafe {
            let layout = Layout::array::<T>(capacity).unwrap();

            match NonNull::new(allocator::alloc(layout) as *mut T) {
                Some(p) => p,
                None => allocator::handle_alloc_error(layout),
            }
        };

        Self {
            ptr,
            len: 0,
            cap: capacity as _,
        }
    }

    fn grow(&mut self) {
        let (new_cap, new_layout) = if self.cap == 0 {
            (1, Layout::array::<T>(1).unwrap())
        } else {
            let new_cap = (2 * self.cap) as usize;
            let new_layout = Layout::array::<T>(new_cap as _).unwrap();
            (new_cap, new_layout)
        };

        assert!(
            new_layout.size() <= u32::MAX as usize,
            "Allocation too large"
        );

        let new_ptr = if self.cap == 0 {
            unsafe { allocator::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap as _).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { allocator::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => allocator::handle_alloc_error(new_layout),
        };
        self.cap = new_cap as _;
    }

    pub fn push(&mut self, elem: T) {
        if self.len == self.cap {
            self.grow();
        }

        unsafe {
            core::ptr::write(self.ptr.as_ptr().add(self.len as _), elem);
        }

        self.len += 1;
    }

    pub const fn len(&self) -> usize {
        self.len as _
    }

    pub const fn capacity(&self) -> usize {
        self.cap as _
    }

    unsafe fn append_elements(&mut self, slice: *const [T]) {}

    pub const fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len()) }
    }

    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len()) }
    }

    pub fn into_boxed_slice(self) -> Box<[T]> {
        todo!()
    }
}

impl<T> core::ops::Deref for SmallVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> core::ops::DerefMut for SmallVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> Drop for SmallVec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe {
                core::ptr::drop_in_place(core::ptr::slice_from_raw_parts_mut(
                    self.ptr.as_ptr(),
                    self.len(),
                ));

                allocator::dealloc(
                    self.ptr.as_ptr().cast(),
                    Layout::array::<T>(self.cap as _).unwrap(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push() {
        let mut buf = SmallVec::new();

        buf.push(1);
        buf.push(2);

        assert_eq!(buf.as_slice(), &[1, 2]);
    }
}
