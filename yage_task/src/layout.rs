macro_rules! leap {
    ($x:expr) => {
        match $x {
            Some(v) => v,
            None => return None,
        }
    };
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ConstLayout {
    size: usize,
    align: usize,
}

impl ConstLayout {
    pub(crate) const unsafe fn from_size_align_unchecked(size: usize, align: usize) -> Self {
        Self { size, align }
    }

    pub(crate) const fn new_for<T>() -> Self {
        // SAFETY: these come directly from a value, so it must be valid
        unsafe {
            Self::from_size_align_unchecked(core::mem::size_of::<T>(), core::mem::align_of::<T>())
        }
    }

    pub(crate) const unsafe fn unionize(field_one: ConstLayout, field_two: ConstLayout) -> Self {
        let (new_size, new_align) = (
            crate::max(field_one.size, field_two.size),
            crate::max(field_one.align, field_two.align),
        );
        // SAFETY: these are both valid, as per the contract
        unsafe { Self::from_size_align_unchecked(new_size, new_align) }
    }

    pub(crate) const unsafe fn into_standard_layout(self) -> core::alloc::Layout {
        unsafe { core::alloc::Layout::from_size_align_unchecked(self.size, self.align) }
    }

    pub(crate) const fn size(&self) -> usize {
        self.size
    }

    pub(crate) const fn align(&self) -> usize {
        self.align
    }

    pub(crate) const fn extend(self, other: ConstLayout) -> Option<(Self, usize)> {
        let new_align = crate::max(self.align, other.align);
        let pad = self.padding_needed_for(new_align);
        let offset = leap!(self.size.checked_add(pad));
        let new_size = leap!(offset.checked_add(other.size));

        if !new_align.is_power_of_two() || new_size > isize::MAX as usize - (new_align - 1) {
            return None;
        }

        // SAFETY: we validated that align is valid, as well as size
        let layout = unsafe { ConstLayout::from_size_align_unchecked(new_size, new_align) };
        Some((layout, offset))
    }

    pub(crate) const fn padding_needed_for(self, align: usize) -> usize {
        let len = self.size;
        let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        len_rounded_up.wrapping_sub(len)
    }
}
