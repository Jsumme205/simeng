pub mod key;

pub(crate) mod sealed {

    pub trait Sealed {}
}

pub unsafe trait VtableMarker {}

macro_rules! impl_vtable {
    ($item:ident) => {
        impl $crate::evt::sealed::Sealed for $item {}

        unsafe impl $crate::evt::VtableMarker for $item {}
    };
}

pub(crate) use impl_vtable as vtable;

pub unsafe trait Listener {
    type Vtable: VtableMarker;
    type Event;

    fn into_vtable(self) -> Self::Vtable;
}
