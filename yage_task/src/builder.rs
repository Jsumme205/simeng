use crate::header::Tag;

pub struct Builder<M = (), E = ()> {
    pub(crate) metadata: M,
    pub(crate) tag: E,
}

impl Builder {
    pub const fn new() -> Self {
        Self {
            metadata: (),
            tag: (),
        }
    }

    pub const fn metadata<M>(self, metadata: M) -> Builder<M> {
        Builder { metadata, tag: () }
    }
}

impl<M> Builder<M> {
    pub fn tag<E>(self, tag: E) -> Builder<M, E> {
        Builder {
            tag,
            metadata: self.metadata,
        }
    }
}

impl<M, E> Builder<M, E> {}
