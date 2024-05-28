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
        let ptr = slot.put(fn_once);

        Self { ptr }
    }

    #[inline(always)]
    pub fn call(self) -> T {
        let ptr: *mut dyn ThunkFn<'slot, T> = self.ptr;
        core::mem::forget(self);

        unsafe { (*ptr).call_once_in_slot() }
    }
}

impl<T> Drop for Thunk<'_, T> {
    fn drop(&mut self) {
        unsafe { core::ptr::drop_in_place(self.ptr) }
    }
}

trait ThunkFn<'slot, T>: FnOnce(&'slot mut Slot) -> T {
    unsafe fn call_once_in_slot(&'slot mut self) -> T;
}

impl<'slot, T, F> ThunkFn<'slot, T> for F
where
    F: FnOnce(&'slot mut Slot) -> T,
{
    unsafe fn call_once_in_slot(&'slot mut self) -> T {
        let (fn_once, slot) = Slot::take(self);

        fn_once(slot)
    }
}
