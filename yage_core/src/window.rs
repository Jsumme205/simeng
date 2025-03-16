use core::marker::PhantomData;

use yage_sys::window::{RawWindow, RawWindowParams};
//use alloc::boxed::Box;

pub struct Window<S = ()> {
    raw: RawWindow,
    metrics: Metrics,
    _marker: PhantomData<S>,
}

impl<S> Window<S> {}

pub struct Metrics {
    last_second: u64,
    frames: u64,
    fps: u64,
    last_frame: u64,
    delta: u64,
    ticks: u64,
    tps: u64,
    tick_remainder: u64,
}
