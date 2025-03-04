use crate::raw::DataLayout;
use core::{marker::PhantomData, ptr::NonNull};

#[cfg(not(feature = "std"))]
use alloc::alloc;

#[cfg(feature = "std")]
use std::alloc;

use crate::error;


/// the id of an component
/// 
/// this is subject to change, but for now it is a `usize`
pub type ComponentId = usize;

/// the context for a rendering pass
///
/// this is the low level version of it, and is very unstable
///
/// always prefer `yage_core::component::RenderContext<S>` instead
pub struct RenderContext<S> {
    _marker: PhantomData<S>,
}

/// the virtual call table for a componnent
///
/// this contains 2 generic parameters
///
/// 1. `S` is the state of the Engine
///
/// 2. `Dyn` is the trait object that implements all of these functions
///
/// =========== Functions ================
///
/// dimensions: unsafe fn(*const ()) -> (u32, u32)
///    returns a tuple of values, as (width, height)
/// 
/// point: unsafe fn(*const ()) -> (u32, u32)
///    returns: tuple, containing the top-leftmost coordinate of the component
/// 
/// draw: unsafe fn(*mut (), *mut RenderContext<S>) -> error::Result<()>
///     draws the intitial componnent, called once at the begining of the component lifecycle
///
/// update: Option<unsafe fn(*mut (), *mut RenderContext) -> error::Result<()>>
///     updates the component.
///     this is wrapped in an `Option`, because this could in theory be nonexstant. 
///     for example, in the case of a static component.
/// 
/// is_dyn: unsafe fn(*const ()) -> bool
///     returns whether or not this component is dynamic
///
/// component_id: unsafe fn(*const ()) -> ComponentId
///     returns: component id of the current object
/// 
/// query_component: unsafe fn(*const (), ComponentId) -> Option<*const Dyn>
///     queries a component with the ID matching the `ComponentId`, or `None` if this isn't the componet that controls it.
///     returns an immutable reference to that object
/// 
/// query_component_mut: unsafe fn(*mut (), ComponentId) -> Option<*mut Dyn> 
///     like `query_component`, but returns a mutable reference to the object instead
///
/// TODO: add drop implementation
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
    /// creates a `ComponentVTable` for a specified type `T`
    /// SAFETY:
    ///     1. the function pointers must point to valid function instances
    ///     2. `T` must be a valid type for creating this VTable
    ///        |-- this must have valid data to convert from `T` to `ComponentVTable`
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

/// Raw, FFI-compatable component
/// this contains 2 fields, a `data` pointer, then a 
/// static reference to a `ComponentVTable`
#[repr(C)]
pub struct RawComponent<S: 'static, Dyn: ?Sized + 'static> {
    data: NonNull<()>,
    vtable: &'static ComponentVTable<S, Dyn>,
}

impl<S: 'static, Dyn: ?Sized + 'static> RawComponent<S, Dyn> {
    /// SAFETY:
    /// 1. `data` must be valid for the specified VTable
    ///
    /// Panics if allocation fails
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
