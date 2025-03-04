use crate::raw::DataLayout;
use core::{marker::PhantomData, ptr::NonNull};

#[cfg(not(feature = "std"))]
use alloc::alloc;

#[cfg(feature = "std")]
use std::alloc;

pub struct RawKeyEvent<'a> {
    _marker: PhantomData<&'a mut crate::glfw_bindings::GLFWwindow>,
}

pub struct KeyVtable {
    pressed: unsafe fn(*mut (), RawKeyEvent<'_>),
    released: unsafe fn(*mut (), RawKeyEvent<'_>),
    held: unsafe fn(*mut (), RawKeyEvent<'_>),
    drop: unsafe fn(*mut ()),
    layout: &'static DataLayout,
}

impl KeyVtable {
    pub const unsafe fn new_for<T>(
        pressed: unsafe fn(*mut (), RawKeyEvent<'_>),
        released: unsafe fn(*mut (), RawKeyEvent<'_>),
        held: unsafe fn(*mut (), RawKeyEvent<'_>),
        drop: unsafe fn(*mut ()),
    ) -> Self {
        Self {
            pressed,
            released,
            held,
            drop,
            layout: &DataLayout {
                size: core::mem::size_of::<T>(),
                align: core::mem::align_of::<T>(),
            },
        }
    }
}

pub struct Keys {
    data: NonNull<()>,
    vtable: &'static KeyVtable,
}

super::vtable!(Keys);

impl Keys {
    pub unsafe fn new<T>(data: T, vtable: &'static KeyVtable) -> Self {
        let d = {
            let layout = core::alloc::Layout::new::<T>();
            match NonNull::new(alloc::alloc(layout) as *mut ()) {
                Some(p) => p,
                None => panic!("pointer was null"),
            }
        };

        d.cast().write(data);
        Self { data: d, vtable }
    }
}

impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            (self.vtable.drop)(self.data.as_ptr());
            alloc::dealloc(self.data.as_ptr().cast(), self.vtable.layout.layout());
        }
    }
}
