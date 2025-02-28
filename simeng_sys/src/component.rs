use crate::raw::DataLayout;
use core::{marker::PhantomData, ptr::NonNull};

#[cfg(not(feature = "std"))]
use alloc::alloc;

#[cfg(feature = "std")]
use std::alloc;

use crate::error;

pub type ComponentId = usize;

pub struct RenderContext<S> {
    _marker: PhantomData<S>,
}

#[repr(C)]
pub struct ComponentVTable<S, Dyn: ?Sized> {
    dimensions: unsafe fn(*const ()) -> (u32, u32),
    point: unsafe fn(*const ()) -> (u32, u32),
    draw: unsafe fn(*mut (), *mut RenderContext<S>) -> error::Result<()>,
    update: Option<unsafe fn(*mut (), *mut RenderContext<S>) -> error::Result<()>>,
    is_dyn: unsafe fn(*const ()) -> bool,
    component_id: unsafe fn(*const ()) -> ComponentId,
    query_component: unsafe fn(*const (), ComponentId) -> Option<*const Dyn>,
    query_component_mut: unsafe fn(*mut (), ComponentId) -> Option<*mut Dyn>,
    metadata: &'static DataLayout,
}

impl<S, Dyn> ComponentVTable<S, Dyn>
where
    Dyn: ?Sized,
{
    pub const unsafe fn new_for<T>(
        dimensions: unsafe fn(*const ()) -> (u32, u32),
        point: unsafe fn(*const ()) -> (u32, u32),
        draw: unsafe fn(*mut (), *mut RenderContext<S>) -> error::Result<()>,
        update: Option<unsafe fn(*mut (), *mut RenderContext<S>) -> error::Result<()>>,
        is_dyn: unsafe fn(*const ()) -> bool,
        component_id: unsafe fn(*const ()) -> ComponentId,
        query_component: unsafe fn(*const (), ComponentId) -> Option<*const Dyn>,
        query_component_mut: unsafe fn(*mut (), ComponentId) -> Option<*mut Dyn>,
    ) -> Self {
        Self {
            dimensions,
            point,
            draw,
            update,
            is_dyn,
            component_id,
            query_component,
            query_component_mut,
            metadata: &DataLayout {
                size: core::mem::size_of::<T>(),
                align: core::mem::align_of::<T>(),
            },
        }
    }
}

#[repr(C)]
pub struct RawComponent<S: 'static, Dyn: ?Sized + 'static> {
    data: NonNull<()>,
    vtable: &'static ComponentVTable<S, Dyn>,
}

impl<S: 'static, Dyn: ?Sized + 'static> RawComponent<S, Dyn> {
    pub unsafe fn new<T>(data: T, vtable: &'static ComponentVTable<S, Dyn>) -> Self {
        let d = {
            let layout = core::alloc::Layout::new::<T>();
            match NonNull::new(alloc::alloc(layout) as *mut ()) {
                Some(p) => p,
                None => panic!("pointer was null"),
            }
        };

        d.cast().write(data);
        Self { data: d, vtable }
    }
}
