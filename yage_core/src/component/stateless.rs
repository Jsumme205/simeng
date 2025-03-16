use core::{
    pin::Pin,
    task::{Context, Poll},
};

use super::BaseComponent;

pub trait Stateless: BaseComponent {
    fn draw(&mut self) -> crate::Result<()>;

    /// This function should only return Poll::Ready() when it (the component) can be dropped
    fn poll_derender(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<crate::Result<()>>;
}

pub trait StatelessDyn: Stateless {
    fn update(&mut self) -> crate::Result<()>;
}
