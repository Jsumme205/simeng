use alloc::sync::Arc;
use core::{
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::NonNull,
    task::{Context, Poll},
};
use yage_util::{
    atomic::{Atomic, AtomicMut},
    list::{Link, LinkedList, Pointers},
};

use super::{BaseComponent, RenderContext};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Lifecycle {
    Init,
    Drawing,
    Idle,
    Updating,
    Derendering,
    Dropping,
}

pub(crate) struct ComponentHandle<S, Dyn: ?Sized + BaseComponent> {
    pointers: UnsafeCell<Pointers<Self>>,
    lifecycle: Cell<Lifecycle>,
    pub(crate) component: Atomic<Box<Dyn>>,
    _marker: PhantomData<S>,
}

impl<S, Dyn> ComponentHandle<S, Dyn>
where
    Dyn: ?Sized + BaseComponent,
{
    /// SAFETY: by the time that `F` finishes running, `MaybeUninit<Box<Dyn>>` must be in a properly
    /// initialized state
    pub(crate) unsafe fn init<F>(init: F) -> Self
    where
        F: FnOnce(&mut MaybeUninit<Box<Dyn>>),
    {
        let mut comp = MaybeUninit::<Box<Dyn>>::uninit();
        init(&mut comp);
        Self {
            pointers: UnsafeCell::new(Pointers::new()),
            lifecycle: Cell::new(Lifecycle::Init),
            component: Atomic::new(unsafe { comp.assume_init() }),
            _marker: PhantomData,
        }
    }

    pub(crate) fn run_cycle<D, U, Dr>(
        &self,
        ctx: &mut RenderContext<S>,
        cx: &mut Context<'_>,
        draw_op: D,
        update_op: U,
        mut derender_op: Dr,
    ) -> Poll<crate::Result<()>>
    where
        D: FnOnce(
            &mut AtomicMut<'_, Box<Dyn>>,
            &mut RenderContext<S>,
            &mut Context<'_>,
        ) -> Poll<crate::Result<()>>,
        U: FnOnce(
            &mut AtomicMut<'_, Box<Dyn>>,
            &mut RenderContext<S>,
            &mut Context<'_>,
        ) -> Poll<crate::Result<()>>,
        Dr: FnMut(
            &mut AtomicMut<'_, Box<Dyn>>,
            &mut RenderContext<S>,
            &mut Context<'_>,
        ) -> Poll<crate::Result<()>>,
    {
        let lifecycle = self.lifecycle.get();
        match lifecycle {
            Lifecycle::Init => {
                self.lifecycle.set(Lifecycle::Drawing);
                // wake as soon as possible, we can probably draw
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Lifecycle::Drawing => {
                // TODO: replace to try_borrow?
                let mut comp = self.component.borrow_mut();
                draw_op(&mut comp, ctx, cx)
            }
            Lifecycle::Idle => {
                self.lifecycle.set(Lifecycle::Updating);
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Lifecycle::Updating => {
                let mut comp = self.component.borrow_mut();
                update_op(&mut comp, ctx, cx)
            }
            Lifecycle::Derendering => {
                let mut comp = self.component.borrow_mut();
                loop {
                    match derender_op(&mut comp, ctx, cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Ok(())) => {
                            self.lifecycle.set(Lifecycle::Dropping);
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        }
                        Poll::Ready(Err(_)) => panic!("an error occured"),
                    }
                }
            }
            Lifecycle::Dropping => Poll::Ready(Ok(())),
        }
    }
}

unsafe impl<S, Dyn> Link for ComponentHandle<S, Dyn>
where
    Dyn: ?Sized + BaseComponent,
{
    type Handle = Arc<ComponentHandle<S, Dyn>>;
    type Target = ComponentHandle<S, Dyn>;

    fn as_raw(handle: &Self::Handle) -> core::ptr::NonNull<Self::Target> {
        let clone = Arc::clone(handle);
        unsafe { NonNull::new_unchecked(Arc::into_raw(clone) as *mut _) }
    }

    unsafe fn from_raw(ptr: core::ptr::NonNull<Self::Target>) -> Self::Handle {
        unsafe { Arc::from_raw(ptr.as_ptr() as *const _) }
    }

    unsafe fn pointers(
        target: core::ptr::NonNull<Self::Target>,
    ) -> core::ptr::NonNull<Pointers<Self::Target>> {
        unsafe { NonNull::new_unchecked(target.as_ref().pointers.get()) }
    }
}

macro_rules! impl_handle {
    (dyn $ty:ident<$state_ty:ident> => $init:expr) => {
        unsafe {
            $crate::component::component_handle::ComponentHandle::<
                $state_ty,
                dyn $ty<State = $state_ty>,
            >::init(|slot| {
                slot.as_mut_ptr().write(::alloc::boxed::Box::new($init));
            })
        }
    };
}

pub(crate) use impl_handle as handle;

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use yage_util::list::LinkedList;

    use crate::component::{BaseComponent, Component};

    use super::ComponentHandle;

    struct State;
    struct Comp;

    impl BaseComponent for Comp {
        fn dimensions(&self) -> crate::Dimensions {
            crate::Dimensions {
                width: 0,
                height: 0,
            }
        }

        fn component_id(&self) -> usize {
            0
        }
    }

    impl Component for Comp {
        type State = State;

        fn draw(
            &mut self,
            ctx: &mut crate::component::RenderContext<Self::State>,
        ) -> crate::Result<()> {
            Ok(())
        }

        fn poll_derender(
            self: core::pin::Pin<&mut Self>,
            ctx: &mut crate::component::RenderContext<Self::State>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<crate::Result<()>> {
            core::task::Poll::Ready(Ok(()))
        }
    }

    fn make_comp_handle() -> ComponentHandle<State, dyn Component<State = State>> {
        super::handle!(dyn Component<State> => Comp)
    }

    fn test_linked_list() {
        let mut list: LinkedList<ComponentHandle<State, dyn Component<State = State>>> =
            LinkedList::new();

        let h1 = Arc::new(make_comp_handle());
        let h2 = Arc::new(make_comp_handle());

        list.push_front(h1);
        list.push_front(h2);

        let v = list.pop_front();
        let v2 = list.pop_front();

        assert!(v.is_some() && v2.is_some())
    }
}
