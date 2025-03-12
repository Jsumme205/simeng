use core::marker::PhantomData;

use yage_sys::component::ComponentVTable;

//#[cfg(feature = "async_support")]
pub mod stateless;
pub mod sync;

mod __glue;

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
    __glue::Valid<Self>: __glue::Subtrait<dyn BaseComponent>,
{
    fn dimensions(&self) -> Dimensions;

    fn topmost_left_point(&self) -> (u32, u32);

    fn component_id(&self) -> yage_sys::component::ComponentId;

    fn query_component(&self, id: yage_sys::component::ComponentId) -> Option<&dyn BaseComponent>;

    fn query_component_mut(
        &mut self,
        id: yage_sys::component::ComponentId,
    ) -> Option<&mut dyn BaseComponent>;
}

pub trait Component: BaseComponent {
    type State;
    fn draw(&mut self, ctx: &mut RenderContext<Self::State>) -> yage_sys::error::Result<()>;
}

pub trait DynamicComponent: Component {
    fn update(
        &mut self,
        ctx: &mut RenderContext<<Self as Component>::State>,
    ) -> yage_sys::error::Result<()>;
}

pub type Vtable<S> = yage_sys::component::ComponentVTable<S, dyn BaseComponent>;
