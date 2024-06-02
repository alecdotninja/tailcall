use crate::slot::{Slot, SlotBox};

pub struct Thunk<'slot, T> {
    thunk_fn: SlotBox<'slot, dyn ThunkFn<'slot, T>>,
}

impl<'slot, T> Thunk<'slot, T> {
    #[inline(always)]
    pub fn new_in<F>(slot: &'slot mut Slot, fn_once: F) -> Self
    where
        F: FnOnce(&'slot mut Slot) -> T + 'slot,
    {
        let fn_once = SlotBox::new_in(slot, fn_once);

        Self {
            // Convert the thin pointer to `F` into a fat pointer to a
            // `dyn ThunkFn`. This is required since stable Rust does not yet
            // support "unsized coercions" on user-defined pointer types.
            #[allow(trivial_casts)]
            thunk_fn: SlotBox::coerce(fn_once, |p| p as _),
        }
    }

    #[inline(always)]
    pub fn call(self) -> T {
        let thunk_fn = SlotBox::leak(self.thunk_fn);

        // SAFETY: `thunk_fn` is in a `Slot` and since this function takes
        // ownership of self, it cannot be called again.
        unsafe { thunk_fn.call_once_in_slot() }
    }
}

trait ThunkFn<'slot, T>: FnOnce(&'slot mut Slot) -> T {
    // SAFETY: This method may only be called once and `self` must be stored in
    // a `Slot`.
    unsafe fn call_once_in_slot(&'slot mut self) -> T;
}

impl<'slot, T, F> ThunkFn<'slot, T> for F
where
    F: FnOnce(&'slot mut Slot) -> T,
{
    unsafe fn call_once_in_slot(&'slot mut self) -> T {
        // SAFETY: Our caller garentees that `self` is stored in a `Slot`.
        let in_slot = unsafe { SlotBox::adopt(self) };
        let (slot, fn_once) = SlotBox::unwrap(in_slot);

        // Call the underlying function with the now empty slot.
        fn_once(slot)
    }
}
