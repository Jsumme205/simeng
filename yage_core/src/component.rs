use core::{
    pin::Pin,
    task::{Context, Poll},
};

use alloc::sync::Arc;
use yage_util::list::LinkedList;

use crate::Dimensions;

mod __glue;
pub(crate) mod component_handle;
pub mod stateless;
pub mod sync;

pub struct RenderContext<S> {
    state: S,
}

pub trait BaseComponent
where
    __glue::Valid<Self>: __glue::Subtrait<dyn BaseComponent>,
{
    fn dimensions(&self) -> Dimensions;

    fn component_id(&self) -> usize;
}

pub trait Component: BaseComponent {
    type State;

    fn draw(&mut self, ctx: &mut RenderContext<Self::State>) -> crate::Result<()>;

    fn poll_derender(
        self: Pin<&mut Self>,
        ctx: &mut RenderContext<Self::State>,
        cx: &mut Context<'_>,
    ) -> Poll<crate::Result<()>>;
}

pub trait DynamicComponent: Component {
    fn redraw(&mut self, ctx: &mut RenderContext<Self::State>) -> crate::Result<()>;
}

pub struct ComponentList<S> {
    inner: LinkedList<component_handle::ComponentHandle<S, dyn Component<State = S>>>,
}

impl<S> ComponentList<S> {
    pub const fn new() -> Self {
        Self {
            inner: LinkedList::new(),
        }
    }

    pub(crate) fn push<C>(&mut self, component: C)
    where
        C: Component<State = S> + 'static,
    {
        let comp = component_handle::handle!(dyn Component<S> => component);
        self.inner.push_front(Arc::new(comp));
    }
}
