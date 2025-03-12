#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "unstable", feature(marker_trait_attr))]

use core::marker::PhantomData;

use window::Window;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std as alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

pub mod component;
pub mod listeners;
pub mod window;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

pub struct Executor;
pub struct Assets;

pub struct EngineBuilder<S> {
    state: Option<S>,
}

pub struct Engine<S> {
    window: Window<S>,
    components: Vec<component::Vtable<S>>,
    state: Option<S>,
    executor: Executor,
    assets: Assets,
}

impl<S> Engine<S> {
    pub fn builder() -> EngineBuilder<S> {
        EngineBuilder { state: None }
    }
}

impl<S> EngineBuilder<S> {
    pub fn with_state<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut Assets) -> S,
    {
        match &mut self.state {
            Some(_) => self,
            None => {
                let state = f(&mut Assets);
                self.state = Some(state);
                self
            }
        }
    }

    pub fn build(self) -> Result<Engine<S>, ()> {
        Ok(Engine {
            window: todo!(),
            components: Vec::new(),
            state: self.state,
            executor: Executor,
            assets: Assets,
        })
    }
}
