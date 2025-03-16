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
    type State;

    fn poll_draw(
        self: Pin<&mut Self>,
        ctx: &mut AsyncRenderContext<'_, '_, Self::State>,
    ) -> Poll<crate::Result<()>>;

    fn poll_derender(
        self: Pin<&mut Self>,
        ctx: &mut AsyncRenderContext<'_, '_, Self::State>,
    ) -> Poll<crate::Result<()>>;
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
    type Output = crate::Result<()>;

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
    ) -> Poll<crate::Result<()>>;
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
    type Output = crate::Result<()>;

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
mod tests {}
