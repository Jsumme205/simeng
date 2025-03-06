use core::marker::PhantomData;

use simeng_sys::component::ComponentVTable;

//#[cfg(feature = "async_support")]
pub mod sync;

#[cfg(feature = "unstable")]
mod __glue;

#[cfg(not(feature = "unstable"))]
mod __glue {
    // this is here in the case that unstable is turned off
    // it makes it so ALL types implement this trait
    // virtually cancelling it out
    use core::marker::PhantomData;
    use super::BaseComponent;

    pub struct Valid<T: ?Sized>(PhantomData<T>);
    pub unsafe trait Subtrait<T: ?Sized> {}

    unsafe impl<T: ?Sized> Subtrait<dyn BaseComponent> for Valid<T> {}
}

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

pub trait BaseComponent 
where
    __glue::Valid<Self>: __glue::Subtrait<dyn BaseComponent>
{
    fn dimensions(&self) -> Dimensions;

    fn topmost_left_point(&self) -> (u32, u32);

    fn component_id(&self) -> simeng_sys::component::ComponentId;

    fn query_component(&self, id: simeng_sys::component::ComponentId) -> Option<&dyn BaseComponent>;

    fn query_component_mut(&mut self, id: simeng_sys::component::ComponentId) -> Option<&mut dyn BaseComponent>;
}

pub trait Component: BaseComponent {
    type State;
    fn draw(&mut self, ctx: &mut RenderContext<Self::State>) -> simeng_sys::error::Result<()>;
}

pub trait DynamicComponent: Component {
    fn update(&mut self, ctx: &mut RenderContext<<Self as Component>::State>) -> simeng_sys::error::Result<()>;
}

pub type Vtable<S> = simeng_sys::component::ComponentVTable<S, dyn Component<State = S>>;
