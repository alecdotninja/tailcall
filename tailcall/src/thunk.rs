use core::{any::type_name, fmt, marker::PhantomData, mem::transmute, ptr::drop_in_place};

use crate::slot::Slot;

const MAX_THUNK_DATA_SIZE: usize = 48;

type ThunkSlot = Slot<MAX_THUNK_DATA_SIZE>;
type CallFn<T> = fn(ThunkSlot) -> T;
type DropInPlaceFn = unsafe fn(*mut ThunkSlot);

#[repr(transparent)]
pub struct Thunk<'a, T = ()> {
    inner: Inner<'a, T>,
}

struct Inner<'a, T> {
    slot: ThunkSlot,
    call_impl: CallFn<T>,
    drop_in_place_impl: DropInPlaceFn,
    _marker: PhantomData<dyn FnOnce() -> T + 'a>,
}

impl<'a, T> Thunk<'a, T> {
    pub const fn new<F>(fn_once: F) -> Self
    where
        F: FnOnce() -> T + 'a,
    {
        Self {
            inner: Inner {
                slot: Slot::new(fn_once),
                call_impl: |slot| {
                    // SAFETY: `slot` is initialized above with `F`.
                    unsafe { slot.into_value::<F>()() }
                },
                drop_in_place_impl: |slot_ptr| {
                    // SAFETY: `slot` is initialized above with `F`.
                    unsafe { drop_in_place(slot_ptr.cast::<F>()) };
                },
                _marker: PhantomData,
            },
        }
    }

    #[inline(always)]
    pub fn call(self) -> T {
        // SAFETY: `Thunk` is a transparent wrapper around `Inner`. This also
        // ensures that the `Drop` impl will not run.
        let Inner {
            slot, call_impl, ..
        } = unsafe { transmute(self) };

        call_impl(slot)
    }
}

impl<T> Drop for Thunk<'_, T> {
    fn drop(&mut self) {
        // SAFETY: We own the slot, and it cannot be used after dropping.
        unsafe { (self.inner.drop_in_place_impl)(&mut self.inner.slot) }
    }
}

impl<T> fmt::Debug for Thunk<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Thunk -> {}", type_name::<T>())
    }
}
