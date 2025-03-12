use super::{BaseComponent, RenderContext};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub struct AsyncRenderContext<'borrow, 'ctx, S> {
    cx: &'borrow mut Context<'ctx>,
    render_context: &'borrow mut RenderContext<S>,
}

impl<'borrow, 'ctx, S> core::ops::Deref for AsyncRenderContext<'borrow, 'ctx, S> {
    type Target = RenderContext<S>;

    fn deref(&self) -> &Self::Target {
        &self.render_context
    }
}

impl<'borrow, 'ctx, S> core::ops::DerefMut for AsyncRenderContext<'borrow, 'ctx, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render_context
    }
}

impl<'borrow, 'ctx, S> AsyncRenderContext<'borrow, 'ctx, S> {
    pub fn ctx(&mut self) -> &mut Context<'ctx> {
        &mut *self.cx
    }
}

pub trait AsyncComponent: BaseComponent {
    type Error;
    type State;

    fn poll_draw(
        self: Pin<&mut Self>,
        ctx: &mut AsyncRenderContext<'_, '_, Self::State>,
    ) -> Poll<Result<(), Self::Error>>;
}

pub trait AsyncComponentExt: AsyncComponent + Unpin {
    fn draw<'a>(
        &'a mut self,
        render_context: &'a mut RenderContext<Self::State>,
    ) -> Draw<'a, Self> {
        Draw::new(self, render_context)
    }
}

pub struct Draw<'a, C: ?Sized + AsyncComponent> {
    component: &'a mut C,
    render_context: &'a mut RenderContext<C::State>,
}

pub(super) struct DrawProjection<'__pin, C: ?Sized + AsyncComponent> {
    component: Pin<&'__pin mut C>,
    render_context: &'__pin mut RenderContext<C::State>,
}

impl<'a, C> Draw<'a, C>
where
    C: AsyncComponent + ?Sized + Unpin,
{
    pub(super) fn new(component: &'a mut C, ctx: &'a mut RenderContext<C::State>) -> Self {
        Self {
            component,
            render_context: ctx,
        }
    }

    pub(super) fn project<'__pin>(self: Pin<&'__pin mut Self>) -> DrawProjection<'__pin, C> {
        let Self {
            component,
            render_context,
        } = self.get_mut();
        DrawProjection {
            component: Pin::new(&mut **component),
            render_context: &mut **render_context,
        }
    }
}

impl<'a, C> Future for Draw<'a, C>
where
    C: AsyncComponent + ?Sized + Unpin,
{
    type Output = Result<(), C::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let DrawProjection {
            component,
            render_context,
        } = self.project();
        let mut ctx = AsyncRenderContext::<'_, '_, _> {
            render_context: &mut *render_context,
            cx: &mut *cx,
        };
        component.poll_draw(&mut ctx)
    }
}

impl<C> AsyncComponentExt for C where C: AsyncComponent + Unpin {}

pub trait AsyncDynamicComponent: AsyncComponent {
    fn poll_update(
        self: Pin<&mut Self>,
        render_context: &mut AsyncRenderContext<'_, '_, <Self as AsyncComponent>::State>,
    ) -> Poll<Result<(), <Self as AsyncComponent>::Error>>;
}

pub trait AsyncDynamicComponentExt: AsyncDynamicComponent + Unpin {
    fn update<'a>(
        &'a mut self,
        ctx: &'a mut RenderContext<<Self as AsyncComponent>::State>,
    ) -> Update<'a, Self> {
        Update::new(self, ctx)
    }
}

impl<C> AsyncDynamicComponentExt for C where C: AsyncDynamicComponent + Unpin {}

pub struct Update<'a, C: ?Sized + AsyncDynamicComponent> {
    component: &'a mut C,
    ctx: &'a mut RenderContext<C::State>,
}

impl<'a, C> Update<'a, C>
where
    C: AsyncDynamicComponent + Unpin + ?Sized,
{
    fn new(component: &'a mut C, ctx: &'a mut RenderContext<C::State>) -> Self {
        Self { component, ctx }
    }

    pub(super) fn __project<'__pin>(self: Pin<&'__pin mut Self>) -> UpdateProjection<'__pin, C> {
        let Self { component, ctx } = self.get_mut();
        UpdateProjection {
            component: Pin::new(&mut **component),
            ctx: &mut **ctx,
        }
    }
}

impl<'a, C> Future for Update<'a, C>
where
    C: AsyncDynamicComponent + Unpin + ?Sized,
{
    type Output = Result<(), C::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let UpdateProjection { component, ctx } = self.__project();
        let mut ctx = AsyncRenderContext {
            cx,
            render_context: &mut *ctx,
        };
        component.poll_update(&mut ctx)
    }
}

pub(super) struct UpdateProjection<'__pin, C: ?Sized + AsyncDynamicComponent> {
    component: Pin<&'__pin mut C>,
    ctx: &'__pin mut RenderContext<C::State>,
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use crate::component::BaseComponent;
    use core::{sync::atomic::AtomicUsize, task::Waker};

    use super::*;

    struct State;

    struct Component;

    impl BaseComponent for Component {
        fn dimensions(&self) -> crate::Dimensions {
            crate::Dimensions {
                width: 0,
                height: 0,
            }
        }

        fn component_id(&self) -> yage_sys::component::ComponentId {
            0
        }

        fn query_component(
            &self,
            id: yage_sys::component::ComponentId,
        ) -> Option<&dyn BaseComponent> {
            None
        }

        fn query_component_mut(
            &mut self,
            id: yage_sys::component::ComponentId,
        ) -> Option<&mut dyn BaseComponent> {
            None
        }

        fn topmost_left_point(&self) -> (u32, u32) {
            (0, 0)
        }
    }

    impl AsyncComponent for Component {
        type Error = ();
        type State = State;

        fn poll_draw(
            self: Pin<&mut Self>,
            ctx: &mut AsyncRenderContext<'_, '_, State>,
        ) -> Poll<Result<(), Self::Error>> {
            println!("drawing...");
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncDynamicComponent for Component {
        fn poll_update(
            self: Pin<&mut Self>,
            ctx: &mut AsyncRenderContext<'_, '_, State>,
        ) -> Poll<Result<(), <Self as AsyncComponent>::Error>> {
            static COUNT: AtomicUsize = AtomicUsize::new(1);
            println!("updating...");
            if COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed) == 1 {
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }

    #[test]
    fn test_dummy_component_draw() {
        let mut ctx = RenderContext::new(State);
        let mut component = Component;

        let mut fut = async { component.draw(&mut ctx).await };
        let mut cx = Context::from_waker(Waker::noop());
        unsafe { assert!(Future::poll(Pin::new_unchecked(&mut fut), &mut cx).is_ready()) }
    }

    #[test]
    fn test_dummy_component_update() {
        let mut ctx = RenderContext::new(State);
        let mut component = Component;

        let fut = async { component.update(&mut ctx).await };
        let mut fut = Box::pin(fut);
        let mut cx = Context::from_waker(Waker::noop());
        let mut count = 0;
        loop {
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(r) => {
                    assert!(count == 1 && r.is_ok());
                    break;
                }
                Poll::Pending => count += 1,
            }
        }
    }
}
