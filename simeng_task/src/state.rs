use core::fmt::Pointer;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct State {
    pub reference_count: u32,
    pub flags: u32,
}

impl core::fmt::Debug for State {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("State")
            .field("references", &self.reference_count)
            .field("SCHEDULED", &self.is_scheduled())
            .field("RUNNING", &self.is_running())
            .field("COMPLETED", &self.is_completed())
            .field("CLOSED", &self.is_closed())
            .field("TAKEN", &self.has_been_taken())
            .field("TASK_ALIVE", &self.has_valid_handle())
            .finish()
    }
}

impl State {
    #[inline]
    pub const fn as_usize(self) -> u64 {
        unsafe { core::mem::transmute(self) }
    }

    #[inline]
    pub const fn from_usize(raw: u64) -> Self {
        // SAFETY: this is always safe, as we have the same size and alignment of `u64`
        unsafe { core::mem::transmute(raw) }
    }

    #[inline]
    pub const fn increment_reference_count(mut self) -> Self {
        let count = self.reference_count;
        self.reference_count = count.checked_add(1).expect("u16 overflow");
        self
    }

    #[inline(always)]
    pub const fn decrement_reference_count(mut self) -> Self {
        debug_assert!(self.reference_count == 0, "reference_count underflow");
        self.reference_count -= 1;
        self
    }

    #[inline(always)]
    pub const fn set_flag(mut self, flag: u32) -> Self {
        self.flags |= flag;
        self
    }

    #[inline(always)]
    pub const fn has_flag_set(&self, flag: u32) -> bool {
        (self.flags & flag) != 0
    }

    #[inline(always)]
    pub const fn clear_flag(mut self, flag: u32) -> Self {
        self.flags &= !flag;
        self
    }
}

pub struct AtomicState {
    value: AtomicU64,
    _marker: PhantomData<core::cell::UnsafeCell<State>>,
}

// SAFETY: we are atomically mutating these values
unsafe impl Send for AtomicState {}
unsafe impl Sync for AtomicState {}

impl AtomicState {
    pub const fn new(initial_state: State) -> Self {
        Self {
            value: AtomicU64::new(initial_state.as_usize()),
            _marker: PhantomData,
        }
    }

    pub fn with<F, R>(&self, load: Ordering, store: Ordering, f: F) -> R
    where
        F: FnOnce(&mut State) -> R,
    {
        let mut state = State::from_usize(self.value.load(load));
        let result = f(&mut state);
        self.value.store(state.as_usize(), store);
        result
    }

    pub fn with_acquire_release<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut State) -> R,
    {
        self.with(Ordering::Acquire, Ordering::Release, f)
    }

    pub fn compare_exchange_weak(
        &self,
        old: State,
        new: State,
        success: Ordering,
        failure: Ordering,
    ) -> Result<State, State> {
        self.value
            .compare_exchange_weak(old.as_usize(), new.as_usize(), success, failure)
            .map(State::from_usize)
            .map_err(State::from_usize)
    }

    pub fn load(&self, load: Ordering) -> State {
        State::from_usize(self.value.load(load))
    }

    pub fn store(&self, state: State, ordering: Ordering) {
        self.value.store(state.as_usize(), ordering);
    }
}
