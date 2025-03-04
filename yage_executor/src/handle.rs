use alloc::sync::Arc;
use core::{marker::PhantomData, sync::atomic::AtomicUsize};
use simeng_task::runnable::Handle;

pub struct ScopeData {
    number_running_tasks: AtomicUsize,
}

pub struct TaskHandle<'scope, T, M> {
    pub inner_handle: Handle<T, M>,
    pub scope_data: Option<Arc<ScopeData>>,
    pub _marker: PhantomData<&'scope ScopeData>,
}

impl<'scope, T, M> TaskHandle<'scope, T, M> {
    pub fn join(self) -> Option<T> {
        let mut this = core::mem::ManuallyDrop::new(self);
        loop {
            match this.inner_handle.poll_output() {
                core::task::Poll::Ready(out) => return out,
                core::task::Poll::Pending => {}
            }
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use simeng_task::builder::Builder;

    fn run_task<M>(task: simeng_task::runnable::Task<M>) -> simeng_task::RawPoll {
        let waker = task.waker();
        let mut cx = core::task::Context::from_waker(&waker);
        task.run(&mut cx)
    }

    #[test]
    #[cfg(test)]
    fn test_join_handle() {
        let (sender, recv) = flume::unbounded();

        let schedule = move |task| sender.send(task).unwrap();

        let (task, handle) = Builder::new().spawn(move |()| async { 1 + 2 }, schedule);
        task.schedule();

        let task = recv.recv().unwrap();
        // HACK: the reference count underflowed, so i'm creating a temporary waker to hopefully keep it alive
        let state = task.state();
        std::println!("state: {state:?}");

        let _ = run_task(task);

        let handle = TaskHandle {
            inner_handle: handle,
            scope_data: None,
            _marker: PhantomData,
        };

        assert!(handle.join().unwrap() == 3)
    }
}
