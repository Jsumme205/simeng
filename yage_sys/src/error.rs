// TODO: add error implementation for #[cfg(feature = "std")]

use core::fmt::Debug;


#[cfg(not(feature = "std"))]
use alloc::boxed;

#[cfg(feature = "std")]
use std::boxed;


/// the kind of error that occured, 
/// this is non-exhaustive, and subject to change in the future
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ErrorKind {
    /// we failed to initialize the GLFw Instance
    FailedInit,
    /// the call to `glfwCreateWindow` returned NULL
    WindowNull,
    /// we failed getting the dimensions from a window
    FailedGettingDims,
    /// other error
    Other,
}

/// an error that occured in GLFW
pub struct GlfwError {
    // the kind of error
    kind: ErrorKind,
    //#[cfg(feature = "")]
    
    payload: Option<boxed::Box<dyn AsRef<str>>>,
}

impl GlfwError {
    /// creates a simple error, with just a `ErrorKind`
    pub const fn simple(kind: ErrorKind) -> Self {
        Self {
            kind,
            //#[cfg(feature = "alloc")]
            payload: None,
        }
    }

    /// creates a error with a kind and a payload, which implements `AsRef<str>`
    //#[cfg(feature = "alloc")]
    pub fn with_payload<P>(kind: ErrorKind, payload: P) -> Self
    where
        P: AsRef<str> + 'static,
    {
        Self {
            kind,
            payload: Some(boxed::Box::new(payload)),
        }
    }

    /// returns the kind of the error.
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl Debug for GlfwError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut dbg = f.debug_struct("GlfwError");
        //#[cfg(feature = "alloc")]
        let dbg = if let Some(ref payload) = self.payload {
            dbg.field("payload", &payload.as_ref().as_ref())
        } else {
            &mut dbg
        };
        dbg.field("kind", &self.kind).finish()
    }
}

pub type Result<T> = core::result::Result<T, GlfwError>;
