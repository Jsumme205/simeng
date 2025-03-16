use core::fmt::{self, Pointer};

#[cfg(feature = "std")]
use alloc::io;
#[cfg(feature = "std")]
use alloc::io::ErrorKind as IoErrorKind;
use yage_sys::error::ErrorKind as SysErrorKind;
use yage_sys::error::GlfwError;

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, PartialOrd, Hash, Debug, Default)]
pub enum ErrorKind {
    System(SysErrorKind),
    IoError(IoErrorKind),
    ComponentDrawError,
    DisabledRt,
    #[default]
    Other,
}

impl From<GlfwError> for Error {
    fn from(value: GlfwError) -> Self {
        Self {
            kind: ErrorKind::System(value.kind()),
            details: None,
        }
    }
}

#[cfg(feature = "std")]
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self {
            kind: ErrorKind::IoError(err.kind()),
            details: None,
        }
    }
}

pub struct Error {
    kind: ErrorKind,
    details: Option<alloc::boxed::Box<dyn core::convert::AsRef<str>>>,
}

impl Error {
    pub const fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            details: None,
        }
    }

    pub fn with_details<T>(mut self, details: T) -> Self
    where
        T: AsRef<str> + 'static,
    {
        self.details = Some(Box::new(details));
        self
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref details) = self.details {
            f.debug_struct("Error")
                .field("kind", &self.kind)
                .field("details", &details.as_ref().as_ref())
                .finish()
        } else {
            f.debug_struct("Error").field("kind", &self.kind).finish()
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
