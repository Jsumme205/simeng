use core::sync::atomic::AtomicIsize;

pub(crate) type BorrowFlag = AtomicIsize;

pub(super) const fn new_borrow_flag() -> BorrowFlag {
    BorrowFlag::new(0)
}
