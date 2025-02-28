#![cfg_attr(not(feature = "std"), no_std)]

use core::{alloc::Layout, ptr::NonNull};
use spin::Mutex;

static ERROR: Mutex<Option<Err>> = Mutex::new(None);

use glfw_bindings::{glfwInit, glfwSetErrorCallback, GLFW_FOCUSED, GLFW_NO_ERROR};
use shader::CompiledShaders;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
use std::alloc as allocator;

#[cfg(not(feature = "std"))]
use alloc::alloc as allocator;

pub mod component;
pub mod error;
pub mod evt;

mod gl_bindings;
mod glfw_bindings;
pub mod raw;
pub mod shader;
pub mod window;

pub fn register_error_callback<T>(cb: T)
where
    T: ErrorCallback,
{
    let mut guard = ERROR.lock();
    *guard = vtable_for(cb);
}

pub fn check_for_errors() {
    let mut ptr: *const i8 = core::ptr::null();
    let error_code = unsafe { glfw_bindings::glfwGetError(&raw mut ptr) };
}

pub fn glfw_init() -> crate::error::Result<()> {
    unsafe {
        glfwInit();
        glfwSetErrorCallback(Some(__detail_error_callback));
        glfw_bindings::glfwWindowHint(glfw_bindings::GLFW_CONTEXT_VERSION_MAJOR as _, 3);
        glfw_bindings::glfwWindowHint(glfw_bindings::GLFW_CONTEXT_VERSION_MINOR as _, 3);
        glfw_bindings::glfwWindowHint(
            glfw_bindings::GLFW_OPENGL_PROFILE as _,
            glfw_bindings::GLFW_OPENGL_CORE_PROFILE as _,
        );
    }

    Ok(())
}

unsafe extern "C" fn __detail_error_callback(code: i32, msg: *const i8) {
    let lock = ERROR.lock();
    match &*lock {
        Some(ref lock) => unsafe {
            (lock.vtable.on_error)(lock.data.as_ptr(), code, core::ffi::CStr::from_ptr(msg))
        },
        None => {
            #[cfg(feature = "default_impls")]
            DefaultErrorCallback.on_error(code, core::ffi::CStr::from_ptr(msg));

            #[cfg(not(feature = "default_impls"))]
            panic!("no callback specified, panicking instead")
        }
    }
}

pub trait ErrorCallback: Send + Sync {
    fn on_error(&self, code: i32, message: &core::ffi::CStr);
}

struct ErrorCallbackVtable {
    on_error: unsafe fn(*const (), code: i32, message: &core::ffi::CStr),
    drop: unsafe fn(*mut ()),
    layout: &'static raw::DataLayout,
}

struct Err {
    data: NonNull<()>,
    vtable: &'static ErrorCallbackVtable,
}

// SAFETY: Error must implement both Send and Sync
unsafe impl Send for Err {}
unsafe impl Sync for Err {}

impl Drop for Err {
    fn drop(&mut self) {
        unsafe {
            (self.vtable.drop)(self.data.as_ptr());
            allocator::dealloc(self.data.as_ptr() as *mut u8, self.vtable.layout.layout());
        }
    }
}

fn vtable_for<T>(error: T) -> Option<Err>
where
    T: ErrorCallback,
{
    let ptr = unsafe {
        let p = allocator::alloc(Layout::new::<T>()) as *mut T;
        p.write(error);
        p
    };
    NonNull::new(ptr as *mut ()).map(|ptr| Err {
        data: ptr,
        vtable: &ErrorCallbackVtable {
            on_error: __detail_on_error::<T>,
            drop: __drop_impl::<T>,
            layout: &raw::DataLayout {
                size: core::mem::size_of::<T>(),
                align: core::mem::align_of::<T>(),
            },
        },
    })
}

unsafe fn __drop_impl<T>(ptr: *mut ()) {
    core::ptr::drop_in_place(ptr as *mut T);
}

unsafe fn __detail_on_error<T>(data: *const (), code: i32, message: &core::ffi::CStr)
where
    T: ErrorCallback,
{
    unsafe {
        T::on_error(&*(data as *const T), code, message);
    }
}

impl ErrorCallbackVtable {
    pub const unsafe fn new_for<T>(
        on_error: unsafe fn(*const (), code: i32, message: &core::ffi::CStr),
    ) -> Self {
        Self {
            on_error,
            drop: __drop_impl::<T>,
            layout: &raw::DataLayout {
                size: core::mem::size_of::<T>(),
                align: core::mem::align_of::<T>(),
            },
        }
    }
}

#[cfg(feature = "default_impls")]
pub struct DefaultErrorCallback;

#[cfg(feature = "default_impls")]
impl ErrorCallback for DefaultErrorCallback {
    fn on_error(&self, code: i32, message: &core::ffi::CStr) {
        println!("{code}: {message:?}")
    }
}
