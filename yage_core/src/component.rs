use core::marker::PhantomData;

use simeng_sys::component::ComponentVTable;

//#[cfg(feature = "async_support")]
pub mod sync;

use crate::Dimensions;

pub struct RenderContext<S> {
    state: S,
    _marker: PhantomData<*mut S>,
}

impl<S> RenderContext<S> {
    pub const fn new(state: S) -> Self {
        Self {
            state,
            _marker: PhantomData,
        }
    }
}

pub trait Component<S> {
    fn dimensions(&self) -> Dimensions;

    fn topmost_left_point(&self) -> (u32, u32);

    fn draw(&mut self, ctx: &mut RenderContext<S>) -> simeng_sys::error::Result<()>;

    fn component_id(&self) -> simeng_sys::component::ComponentId;

    fn query_component(&self, id: simeng_sys::component::ComponentId) -> Option<&dyn Component<S>>;

    fn query_component_mut(
        &mut self,
        id: simeng_sys::component::ComponentId,
    ) -> Option<&mut dyn Component<S>>;

    fn __as_dyn_component(&self) -> Option<&dyn DynamicComponent<S>> {
        None
    }

    fn __as_dyn_component_mut(&mut self) -> Option<&mut dyn DynamicComponent<S>> {
        None
    }
}

pub trait DynamicComponent<S>: Component<S> {
    fn update(&mut self, ctx: &mut RenderContext<S>) -> simeng_sys::error::Result<()>;
}

pub type Vtable<S> = simeng_sys::component::ComponentVTable<S, dyn Component<S>>;
