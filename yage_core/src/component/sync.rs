use super::{RenderContext, BaseComponent};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub struct AsyncRenderContext<'a, S> {
    cx: &'a mut Context<'a>,
    render_context: &'a mut RenderContext<S>
}

impl<'a, S> core::ops::Deref for AsyncRenderContext<'a, S> {
    type Target = RenderContext<S>;

    fn deref(&self) -> &Self::Target {
        &self.render_context
    }
}

impl<'a, S> core::ops::DerefMut for AsyncRenderContext<'a, S> {
   fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render_context
   }
}

pub trait AsyncComponent: BaseComponent  {
    type Error;
    type State;

    fn poll_draw(
        self: Pin<&mut Self>,
        ctx: &mut AsyncRenderContext<'_, Self::State>,
    ) -> Poll<Result<(), Self::Error>>;
}

pub trait AsyncComponentExt: AsyncComponent + Unpin {
    fn draw<'a>(&'a mut self, render_context: &'a mut RenderContext<Self::State>) -> Draw<'a, Self> {
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

    pub(super) fn project<'__pin>(self: Pin<&'__pin mut Self>) -> DrawProjection<'__pin, C, S> {
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
        let mut ctx = AsyncRenderContext {
            render_context: &mut *render_context,
            cx
        }
        component.poll_draw(&mut ctx)
    }
}

impl<C, S> AsyncComponentExt<S> for C where C: AsyncComponent<S> + Unpin {}

pub trait AsyncDynamicComponent<S>: AsyncComponent<S> {
    fn poll_update(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        render_context: &mut RenderContext<S>,
    ) -> Poll<Result<(), <Self as AsyncComponent<S>>::Error>>;
}

pub trait AsyncDynamicComponentExt<S>: AsyncDynamicComponent<S> + Unpin {
    fn update<'a>(&'a mut self, ctx: &'a mut RenderContext<S>) -> Update<'a, S, Self> {
        Update::new(self, ctx)
    }
}

impl<S, C> AsyncDynamicComponentExt<S> for C where C: AsyncDynamicComponent<S> + Unpin {}

pub struct Update<'a, S, C: ?Sized> {
    component: &'a mut C,
    ctx: &'a mut RenderContext<S>,
}

impl<'a, C, S> Update<'a, S, C>
where
    C: AsyncDynamicComponent<S> + Unpin + ?Sized,
{
    fn new(component: &'a mut C, ctx: &'a mut RenderContext<S>) -> Self {
        Self { component, ctx }
    }

    pub(super) fn __project<'__pin>(self: Pin<&'__pin mut Self>) -> UpdateProjection<'__pin, C, S> {
        let Self { component, ctx } = self.get_mut();
        UpdateProjection {
            component: Pin::new(&mut **component),
            ctx: &mut **ctx,
        }
    }
}

impl<'a, S, C> Future for Update<'a, S, C>
where
    C: AsyncDynamicComponent<S> + Unpin + ?Sized,
{
    type Output = Result<(), C::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let UpdateProjection { component, ctx } = self.__project();
        component.poll_update(cx, ctx)
    }
}

pub(super) struct UpdateProjection<'__pin, C: ?Sized, S> {
    component: Pin<&'__pin mut C>,
    ctx: &'__pin mut RenderContext<S>,
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use core::{sync::atomic::AtomicUsize, task::Waker};

    use super::*;

    struct State;

    struct Component;

    impl AsyncComponent<State> for Component {
        type Error = ();

        fn poll_draw(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _render_context: &mut RenderContext<State>,
        ) -> Poll<Result<(), Self::Error>> {
            println!("drawing...");
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncDynamicComponent<State> for Component {
        fn poll_update(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _render_context: &mut RenderContext<State>,
        ) -> Poll<Result<(), <Self as AsyncComponent<State>>::Error>> {
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
