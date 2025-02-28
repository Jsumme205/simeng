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

pub struct RawWindow {
    handle: NonNull<GLFWwindow>,
}

impl RawWindow {
    pub fn create(
        RawWindowParams {
            width,
            height,
            name,
            key_handler,
        }: RawWindowParams,
    ) -> error::Result<Self> {
        let name = name.map(|n| n.as_ptr()).unwrap_or(core::ptr::null());
        let handle = unsafe {
            let w = glfwCreateWindow(
                width as _,
                height as _,
                name,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );

            if w.is_null() {
                return Err(error::GlfwError::simple(error::ErrorKind::WindowNull));
            }

            glfwMakeContextCurrent(w);

            NonNull::new_unchecked(w)
        };
        Ok(Self { handle })
    }

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

pub struct RawWindowParams {
    pub width: u32,
    pub height: u32,
    pub name: Option<CString>,
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
