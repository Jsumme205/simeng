use core::fmt::Debug;

#[cfg(not(feature = "std"))]
use alloc::boxed;

#[cfg(feature = "std")]
use std::boxed;

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ErrorKind {
    FailedInit,
    WindowNull,
    FailedGettingDims,
    Other,
}

pub struct GlfwError {
    kind: ErrorKind,
    //#[cfg(feature = "")]
    payload: Option<boxed::Box<dyn AsRef<str>>>,
}

impl GlfwError {
    pub const fn simple(kind: ErrorKind) -> Self {
        Self {
            kind,
            //#[cfg(feature = "alloc")]
            payload: None,
        }
    }

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
