use crate::slot::Slot;

pub struct Thunk<'slot, T> {
    ptr: &'slot mut dyn ThunkFn<'slot, T>,
}

impl<'slot, T> Thunk<'slot, T> {
    #[inline(always)]
    pub fn new_in<F>(slot: &'slot mut Slot, fn_once: F) -> Self
    where
        F: FnOnce(&'slot mut Slot) -> T + 'slot,
    {
        Self {
            ptr: slot.cast().write(fn_once),
        }
    }

    #[inline(always)]
    pub fn call(self) -> T {
        let ptr: *mut dyn ThunkFn<'slot, T> = self.ptr;
        core::mem::forget(self);

        // SAFETY: The only way to create a `Thunk` is through `Thunk::new_in`
        // which stores the value in a `Slot`. Additionally, we just forgot
        // `self`, so we know that it is impossible to call this method again.
        unsafe { (*ptr).call_once_in_slot() }
    }
}

impl<T> Drop for Thunk<'_, T> {
    fn drop(&mut self) {
        // SAFETY: The owned value was stored in a `Slot` which does not drop,
        // and this struct has a unique pointer to the value there.
        unsafe { core::ptr::drop_in_place(self.ptr) }
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
        // SAFETY: Our caller guarantees that `self` is currently in a `Slot`,
        // and `Slot` guarantees that it is safe to transmute between `&mut F`
        // and `&mut Slot`.
        let slot: &'slot mut Slot = unsafe { core::mem::transmute(self) };

        // SAFETY: We know that there is a `F` in the slot because this method
        // was just called on it. Although the bits remain the same, logically,
        // `fn_once` has been moved *out* of the slot beyond this point.
        let fn_once: F = unsafe { slot.cast().assume_init_read() };

        // Call the underlying function with the now empty slot.
        fn_once(slot)
    }
}
