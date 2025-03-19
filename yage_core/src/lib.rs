#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "unstable", feature(marker_trait_attr))]

use core::marker::PhantomData;

use component::ComponentList;
use window::Window;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std as alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

pub mod component;

pub mod listeners;
pub mod sync;
pub mod window;

pub mod errors;

pub use errors::Result;
use yage_util::list::LinkedList;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

pub struct Executor;
pub struct Assets;

pub struct EngineBuilder<'a, S> {
    state: Option<Box<dyn Future<Output = crate::Result<S>> + 'a>>,
}

pub struct Engine<S> {
    window: Window<S>,
    state: Option<S>,
    executor: Executor,
    assets: Assets,
    components: ComponentList<S>,
}

impl<S> Engine<S> {
    pub fn builder() -> EngineBuilder<S> {
        EngineBuilder { state: None }
    }
}

impl<S> EngineBuilder<S> {
    pub fn with_state<F>(mut self, f: F) -> Self
    where
        F: AsyncFnOnce(&mut Assets) -> S,
    {
        match self.state {
            Some(_) => self,
            None => {
                let state = f(&mut Assets);
                self.state = Some(state);
                self
            }
        }
    }

    pub fn try_with_state<F>(mut self, f: F) -> Self 
    where 
        F: AsyncFnOnce(&mut Assets) -> crate::Result<S>
    {
        match self.state {
            Some(_) => self,
            None => {
                let state = f(&mut Assets);
                self.state = Some(state);
                self
            }
        }
    }
    

    pub fn build(self) -> crate::Result<Engine<S>> {
        let executor = Executor;
        let state = self.state;
        let h = executor.spawn(async move {
            match self.state {
                Some(state) => {
                    state.await?
                },
                None => {
                    Err(crate::Error::new(crate::errors::ErrorKind::StateNotInitialized))
                }
            }
        });
        
        let state = h.join()?;
        
        Ok(Engine {
            window: todo!(),
            state: Some(state),
            executor,
            assets: Assets,
        })
    }
}
