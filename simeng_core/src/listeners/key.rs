use simeng_sys::evt::{
    key::{KeyVtable, Keys, RawKeyEvent},
    Listener,
};

pub struct KeyEvent {}

impl KeyEvent {
    fn __from_sys(_: RawKeyEvent<'_>) -> Self {
        Self {}
    }
}

pub trait KeyListener {
    fn on_key_pressed(&mut self, e: KeyEvent);

    fn on_key_released(&mut self, e: KeyEvent);

    fn on_key_held(&mut self, e: KeyEvent);
}

pub(crate) struct KeyHandler<L: KeyListener> {
    inner: L,
}

impl<L: KeyListener> KeyHandler<L> {
    const VTABLE: KeyVtable = unsafe {
        KeyVtable::new_for::<L>(
            __detail_pressed::<L>,
            __detail_released::<L>,
            __detail_held::<L>,
            super::__detail_drop::<L>,
        )
    };
}

unsafe impl<L: KeyListener> Listener for KeyHandler<L> {
    type Vtable = Keys;
    type Event = KeyEvent;

    fn into_vtable(self) -> Self::Vtable {
        unsafe { Keys::new(self.inner, &Self::VTABLE) }
    }
}

unsafe fn __detail_pressed<T>(data: *mut (), e: RawKeyEvent<'_>)
where
    T: KeyListener,
{
    T::on_key_pressed(&mut *(data as *mut T), KeyEvent::__from_sys(e));
}

unsafe fn __detail_released<T>(data: *mut (), e: RawKeyEvent<'_>)
where
    T: KeyListener,
{
    T::on_key_released(&mut *(data as *mut T), KeyEvent::__from_sys(e));
}

unsafe fn __detail_held<T>(data: *mut (), e: RawKeyEvent<'_>)
where
    T: KeyListener,
{
    T::on_key_held(&mut *(data as *mut T), KeyEvent::__from_sys(e));
}
