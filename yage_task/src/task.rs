use core::marker::PhantomData;
use core::ptr::NonNull;

pub struct Task<Meta = ()> {
    pub(crate) ptr: NonNull<()>,
    pub(crate) _marker: PhantomData<Meta>,
}
