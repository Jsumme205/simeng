use core::cell::UnsafeCell;
use core::marker::{PhantomData, PhantomPinned};
use core::mem::ManuallyDrop;
use core::ops::Neg;
use core::ptr::NonNull;

pub unsafe trait Link {
    type Handle;
    type Target: ?Sized;

    fn as_raw(handle: &Self::Handle) -> NonNull<Self::Target>;

    unsafe fn from_raw(ptr: NonNull<Self::Target>) -> Self::Handle;

    unsafe fn pointers(target: NonNull<Self::Target>) -> NonNull<Pointers<Self::Target>>;
}

pub struct Pointers<T: ?Sized> {
    inner: UnsafeCell<PointersInner<T>>,
}

impl<T: ?Sized> Pointers<T> {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(PointersInner {
                prev: None,
                next: None,
                _pin: PhantomPinned,
            }),
        }
    }

    pub(crate) const fn get_prev(&self) -> Option<NonNull<T>> {
        unsafe { (&raw const (*self.inner.get()).prev).read() }
    }

    pub(crate) const fn get_next(&self) -> Option<NonNull<T>> {
        unsafe { (&raw const (*self.inner.get()).next).read() }
    }

    const fn set_prev(&mut self, val: Option<NonNull<T>>) {
        unsafe {
            (&raw mut (*self.inner.get()).next).write(val);
        }
    }

    const fn set_next(&mut self, val: Option<NonNull<T>>) {
        unsafe {
            (&raw mut (*self.inner.get()).next).write(val);
        }
    }
}

unsafe impl<T: ?Sized> Send for Pointers<T> where T: Send {}
unsafe impl<T: ?Sized> Sync for Pointers<T> where T: Sync {}

struct PointersInner<T: ?Sized> {
    prev: Option<NonNull<T>>,
    next: Option<NonNull<T>>,
    _pin: PhantomPinned,
}

pub struct LinkedList<L>
where
    L: Link,
{
    head: Option<NonNull<L::Target>>,
    tail: Option<NonNull<L::Target>>,
    _marker: PhantomData<*const L>,
}

impl<L> LinkedList<L>
where
    L: Link,
{
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
            _marker: PhantomData,
        }
    }

    pub fn push_front(&mut self, val: L::Handle) {
        let val = ManuallyDrop::new(val);
        let ptr = L::as_raw(&val);
        assert_ne!(self.head, Some(ptr));

        unsafe {
            L::pointers(ptr).as_mut().set_next(self.head);
            L::pointers(ptr).as_mut().set_prev(None);

            if let Some(head) = self.head {
                L::pointers(head).as_mut().set_prev(Some(ptr));
            }

            self.head = Some(ptr);

            if self.tail.is_none() {
                self.tail = Some(ptr)
            }
        }
    }

    pub fn pop_front(&mut self) -> Option<L::Handle> {
        unsafe {
            let head = self.head?;
            self.head = L::pointers(head).as_ref().get_next();
            if let Some(new_head) = L::pointers(head).as_ref().get_next() {
                L::pointers(new_head).as_mut().set_prev(None);
            } else {
                self.tail = None;
            }

            L::pointers(head).as_mut().set_prev(None);
            L::pointers(head).as_mut().set_next(None);

            Some(L::from_raw(head))
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use core::pin::Pin;

    use super::*;

    #[repr(C)]
    struct Entry {
        pointers: Pointers<Self>,
        val: i32,
    }

    unsafe impl<'a> Link for &'a Entry {
        type Handle = Pin<&'a Entry>;
        type Target = Entry;

        fn as_raw(handle: &Self::Handle) -> NonNull<Self::Target> {
            NonNull::from(handle.get_ref())
        }

        unsafe fn from_raw(ptr: NonNull<Self::Target>) -> Self::Handle {
            unsafe { Pin::new_unchecked(&*ptr.as_ptr()) }
        }

        unsafe fn pointers(target: NonNull<Self::Target>) -> NonNull<Pointers<Self::Target>> {
            target.cast()
        }
    }

    fn entry(val: i32) -> Pin<Box<Entry>> {
        Box::pin(Entry {
            pointers: Pointers::new(),
            val,
        })
    }

    #[test]
    fn test_linked_list() {
        let mut list = LinkedList::<&Entry>::new();
        let e1 = entry(0);
        let e2 = entry(1);
        list.push_front(e1.as_ref());
        list.push_front(e2.as_ref());

        let v = list.pop_front();
        assert!(v.map(|v| v.val) == Some(e2.as_ref().val))
    }
}
