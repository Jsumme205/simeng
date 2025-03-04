use core::alloc::Layout;

#[derive(Debug, Clone, Copy)]
pub struct DataLayout {
    pub(crate) size: usize,
    pub(crate) align: usize,
}

impl DataLayout {
    pub const unsafe fn from_size_align_unchecked(size: usize, align: usize) -> Self {
        Self { size, align }
    }

    pub const fn layout(self) -> Layout {
        unsafe { Layout::from_size_align_unchecked(self.size, self.align) }
    }
}
