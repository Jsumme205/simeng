use alloc::boxed::Box;

use crate::{
    RawTask, Schedule,
    runnable::{Handle, Task},
};

pub struct Builder<M = ()> {
    metadata: M,
}

impl Builder<()> {
    pub const fn new() -> Self {
        Self { metadata: () }
    }

    pub fn with_metadata<M>(self, metadata: M) -> Builder<M> {
        Builder { metadata }
    }
}

impl<M> Builder<M> {
    unsafe fn spawn_unchecked_<'a, F, Fut, S>(
        self,
        future: F,
        schedule: S,
    ) -> (Task<M>, Handle<Fut::Output, M>)
    where
        F: FnOnce(&'a M) -> Fut,
        Fut: Future + 'a,
        S: Schedule<M>,
        M: 'a,
    {
        let ptr = if core::mem::size_of::<Fut>() >= 2048 {
            let future = |meta| {
                let f = future(meta);
                Box::pin(f)
            };

            RawTask::<M, _, S>::allocate(future, schedule, self.metadata)
        } else {
            RawTask::<M, Fut, S>::allocate::<'a, F>(future, schedule, self.metadata)
        };

        let runnable = unsafe { Task::from_ptr(ptr) };
        let handle = Handle {
            ptr,

            _marker_types: core::marker::PhantomData,
        };
        (runnable, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::{
        assert_eq,
        pin::Pin,
        task::{Context, Poll, Waker},
    };

    use crate::RawPoll;

    use super::*;

    #[test]
    fn test_builder() {
        let (s, r) = flume::unbounded();

        let (t, mut h) = unsafe {
            Builder::new().spawn_unchecked_(|_| async { 1 + 2 }, move |r| s.send(r).unwrap())
        };
        t.schedule();

        let t = r.recv().unwrap();

        let waker = t.waker();
        let mut cx = Context::from_waker(&waker);
        assert!(t.run(&mut cx) == RawPoll::Complete);
        assert_eq!(
            Pin::new(&mut h).poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(3)
        )
    }
}
