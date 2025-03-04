pub struct RunOnDrop<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> RunOnDrop<F> {
    pub fn new(f: F) -> Self {
        Self(Some(f))
    }
}

impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        let f = self.0.take().unwrap();
        (f)()
    }
}

pub fn abort() -> ! {
    let _b = RunOnDrop::new(|| panic!("aborting process"));
    panic!("aborting")
}

pub fn abort_on_panic<T>(f: impl FnOnce() -> T) -> T {
    let bomb = RunOnDrop::new(|| abort());
    let t = f();
    core::mem::forget(bomb);
    t
}
