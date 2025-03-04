#[cfg(not(feature = "std"))]
use alloc::ffi::CString;

#[cfg(feature = "std")]
use std::ffi::CString;

use crate::{
    error, evt,
    glfw_bindings::{
        glfwCreateWindow, glfwMakeContextCurrent, glfwPollEvents, glfwSwapBuffers,
        glfwWindowShouldClose, GLFWwindow, GLFW_TRUE,
    },
};
use core::ptr::NonNull;


/// Abstraction over a `GLFWindow`
///
/// This is meant to be a very low level abstraction over the object
/// and unless you need a very fine-tuned control, you should be using higher-level API's 
/// such as `yage_core::window::Window`
pub struct RawWindow {
    handle: NonNull<GLFWwindow>,
}

impl RawWindow {

    /// creates a new window, and sets it as the current context
    ///
    /// this is a safe function because of prechecks and post-checks
    pub fn create(
        RawWindowParams {
            width,
            height,
            name,
            key_handler,
        }: RawWindowParams,
    ) -> error::Result<Self> {
        // TODO: change this to something else
        let name = name.map(|n| n.as_ptr()).unwrap_or(core::ptr::null());
        let handle = unsafe {
            // SAFETY: we have valid argments
            let w = glfwCreateWindow(
                width as _,
                height as _,
                name,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );

            // simple null-checking
            if w.is_null() {
                return Err(error::GlfwError::simple(error::ErrorKind::WindowNull));
            }

            // SAFETY: we made sure that the window is created correctly
            glfwMakeContextCurrent(w);

            // SAFETY: this isn't null, becase of our check up there
            NonNull::new_unchecked(w)
        };
        Ok(Self { handle })
    }

    /// runs a loop, polling I/O events and swapping buffers as needed
    /// SAFETY: 
    ///
    /// 1. `F` must not destroy the handle to the window unless there is an error
    /// TODO
    pub unsafe fn main_loop<F>(&mut self, mut f: F) -> error::Result<()>
    where
        F: FnMut(NonNull<GLFWwindow>) -> error::Result<()>,
    {
        while glfwWindowShouldClose(self.handle.as_ptr()) != GLFW_TRUE as _ {
            f(self.handle)?;
            glfwPollEvents();
            glfwSwapBuffers(self.handle.as_ptr());
        }
        Ok(())
    }
}

/// Parameters for a `RawWindow`
pub struct RawWindowParams {
    /// width of a window
    pub width: u32,
    /// height if the window,
    pub height: u32,
    /// name of the window, or `None` if no name specified
    pub name: Option<CString>,
    /// key_handler, or `None` if the default key handler is being used
    pub key_handler: Option<evt::key::Keys>,
}

unsafe fn test() {
    let mut window = RawWindow::create(RawWindowParams {
        width: 200,
        height: 200,
        name: None,
        key_handler: None,
    })
    .unwrap();

    //window.main_loop(|handle| Ok(()))
}
