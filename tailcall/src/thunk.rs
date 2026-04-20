//! Type-erased `FnOnce` storage for the trampoline runtime.
//!
//! A [`crate::thunk::Thunk`] stores the captured data for a single `FnOnce` in a fixed-size stack
//! slot together with the function pointers needed to either call it or drop it in place.

use core::{any::type_name, fmt, marker::PhantomData, mem::ManuallyDrop, ptr::{drop_in_place, read}};

use crate::slot::Slot;

const MAX_THUNK_DATA_SIZE: usize = 48;

type ThunkSlot = Slot<MAX_THUNK_DATA_SIZE>;
type CallFn<T> = fn(ThunkSlot) -> T;
type DropInPlaceFn = unsafe fn(*mut ThunkSlot);

#[repr(transparent)]
/// A type-erased `FnOnce` stored in the trampoline runtime's stack slot.
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
    /// Creates a new thunk from a `FnOnce`.
    ///
    /// The closure's captured state is stored inline in a fixed-size slot. Construction will panic
    /// if the closure's size or alignment exceeds the slot budget chosen by the runtime.
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
    /// Calls the stored `FnOnce`, consuming the thunk in the process.
    ///
    /// Because the thunk owns a `FnOnce`, it can only be called once.
    pub fn call(self) -> T {
        let this = ManuallyDrop::new(self);

        // SAFETY: `this` will not be dropped, so moving `inner` out cannot cause `Thunk::drop`
        // to run after the closure has been taken from the slot.
        let Inner {
            slot, call_impl, ..
        } = unsafe { read(&this.inner) };

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
