use crate::event_loop;
use std::io;

pub trait Notifier {
    fn register(
        &mut self,
        registry: &event_loop::Registry,
        token: crate::Token,
        interests: crate::Interest,
    ) -> io::Result<()>;

    fn reregister(
        &mut self,
        registry: &event_loop::Registry,
        token: crate::Token,
        interests: crate::Interest,
    ) -> io::Result<()>;

    fn deregister(&mut self, registry: &event_loop::Registry) -> io::Result<()>;
}

impl<T> Notifier for Box<T>
where
    T: ?Sized + Notifier,
{
    fn register(
        &mut self,
        registry: &event_loop::Registry,
        token: crate::Token,
        interests: crate::Interest,
    ) -> io::Result<()> {
        (**self).register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &event_loop::Registry,
        token: crate::Token,
        interests: crate::Interest,
    ) -> io::Result<()> {
        (**self).reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &event_loop::Registry) -> io::Result<()> {
        (**self).deregister(registry)
    }
}

impl<T> Notifier for &mut T
where
    T: ?Sized + Notifier,
{
    fn register(
        &mut self,
        registry: &event_loop::Registry,
        token: crate::Token,
        interests: crate::Interest,
    ) -> io::Result<()> {
        T::register(&mut **self, registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &event_loop::Registry,
        token: crate::Token,
        interests: crate::Interest,
    ) -> io::Result<()> {
        T::reregister(&mut **self, registry, token, interests)
    }

    fn deregister(&mut self, registry: &event_loop::Registry) -> io::Result<()> {
        T::deregister(&mut **self, registry)
    }
}
