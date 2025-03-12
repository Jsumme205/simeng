/// Glue module for requiring types to implement a subtrait
///
///

#[cfg(feature = "unstable")]
pub(super) mod __detail {

    use crate::component::{sync::AsyncComponent, BaseComponent, Component};
    use core::marker::PhantomData;

    /// this marker trait is used to indicate that a certain type is a subtrait of
    /// trait `T`
    #[marker]
    pub unsafe trait Subtrait<T: ?Sized> {}

    pub struct Valid<T: ?Sized>(PhantomData<T>);

    unsafe impl<T> Subtrait<dyn BaseComponent> for Valid<T> where T: ?Sized + Component {}
    unsafe impl<T> Subtrait<dyn BaseComponent> for Valid<T> where T: ?Sized + AsyncComponent {}
}

#[cfg(not(feature = "unstable"))]
pub(super) mod __detail {

    use core::marker::PhantomData;

    use crate::component::BaseComponent;

    pub unsafe trait Subtrait<T: ?Sized> {}

    pub struct Valid<T: ?Sized>(PhantomData<T>);

    unsafe impl<T> Subtrait<dyn BaseComponent> for Valid<T> where T: ?Sized {}
}

#[cfg(feature = "unstable")]
pub(super) use __detail::*;

#[cfg(not(feature = "unstable"))]
pub(super) use __detail::*;
