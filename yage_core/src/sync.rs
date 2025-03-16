use yage_executor::{Executor, NotThreadSafe};
use yage_util::list::LinkedList;

use crate::component::{component_handle::ComponentHandle, sync::AsyncComponent};

type AsyncComponentHandle<S> = ComponentHandle<S, dyn AsyncComponent<State = S>>;

pub(crate) enum Async<S> {
    Enabled {
        executor: Executor<NotThreadSafe>,
        components: LinkedList<AsyncComponentHandle<S>>,
    },
    Disabled,
}

impl<S> Async<S> {
    pub const fn disabled() -> Self {
        Self::Disabled
    }

    pub fn enabled() -> Self {
        Self::Enabled {
            executor: Executor::new_unsync(),
            components: LinkedList::new(),
        }
    }
}
