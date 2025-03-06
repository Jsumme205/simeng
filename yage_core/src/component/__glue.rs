
#![cfg(feature = "unstable")]


use core::marker::PhantomData;

use crate::component::{BaseComponent, Component, sync::AsyncComponent, stateless::StatelessComponent};

#[marker] pub unsafe trait Subtrait<T: ?Sized> {}

pub struct Valid<T: ?Sized>(PhantomData<T>);

unsafe impl<T: ?Sized + Component> Subtrait<dyn BaseComponent> for Valid<T> {}
unsafe impl<T: ?Sized + AsyncComponent> Subtrait<dyn BaseComponent> for Valid<T> {}
unsafe impl<T: ?Sized + StatelessComponent> Subtrait<dyn BaseComponent> for Valid<T> {}



