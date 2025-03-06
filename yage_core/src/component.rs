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

pub trait BaseComponent {
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
