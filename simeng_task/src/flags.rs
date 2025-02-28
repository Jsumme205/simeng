pub const SCHEDULED: u32 = 1 << 0;
pub const RUNNING: u32 = 1 << 1;
pub const COMPLETED: u32 = 1 << 2;
pub const CLOSED: u32 = 1 << 3;
pub const CANCELLED: u32 = 1 << 4;
pub const TAKEN: u32 = 1 << 5;
pub const TASK_ALIVE: u32 = 1 << 6;

impl crate::state::State {
    pub const fn is_completed(&self) -> bool {
        self.has_flag_set(COMPLETED)
    }

    pub const fn is_closed(&self) -> bool {
        self.has_flag_set(CLOSED)
    }

    pub const fn is_scheduled(&self) -> bool {
        self.has_flag_set(SCHEDULED)
    }

    pub const fn is_running(&self) -> bool {
        self.has_flag_set(RUNNING)
    }
}
