pub mod key;

pub(super) unsafe fn __detail_drop<T>(data: *mut ()) {
    core::ptr::drop_in_place(data as *mut T);
}
