//! Type-erased `FnOnce` storage for the thunk runtime.
//!
//! An [`ErasedFnOnce`] stores the captured data for a single `FnOnce` in a fixed-size stack slot
//! together with the function pointers needed to either call it or drop it in place.

use core::{
    any::type_name,
    fmt,
    marker::PhantomData,
    mem::ManuallyDrop,
    ptr::{drop_in_place, read},
};

use super::slot::Slot;

// On 64-bit targets, 48 bytes of inline capture storage is the largest budget that still keeps
// the full public `Thunk` representation at 64 bytes once the slot's 16-byte alignment and the
// surrounding function pointers are accounted for.
const MAX_THUNK_DATA_SIZE: usize = 48;

type ErasedFnOnceSlot = Slot<MAX_THUNK_DATA_SIZE>;
type CallFn<T> = fn(ErasedFnOnceSlot) -> T;
type DropInPlaceFn = unsafe fn(*mut ErasedFnOnceSlot);

#[repr(transparent)]
/// A type-erased `FnOnce` stored in the runtime's stack slot.
pub(crate) struct ErasedFnOnce<'a, T = ()> {
    inner: Inner<'a, T>,
}

/// Owns the full erased `FnOnce` representation.
///
/// This inner struct holds the inline capture storage together with the function pointers needed
/// to either call the erased closure or drop its captures in place. Keeping that state in a single
/// field lets [`ErasedFnOnce`] move it out during `call()` while still preserving normal drop
/// semantics when the erased closure is never invoked.
struct Inner<'a, T> {
    slot: ErasedFnOnceSlot,
    call_impl: CallFn<T>,
    drop_in_place_impl: DropInPlaceFn,
    _marker: PhantomData<dyn FnOnce() -> T + 'a>,
}

impl<'a, T> ErasedFnOnce<'a, T> {
    /// Creates a new erased thunk from a `FnOnce`.
    ///
    /// The closure's captured state is stored inline in a fixed-size slot. Construction will panic
    /// if the closure's size or alignment exceeds the slot budget chosen by the runtime.
    pub(crate) const fn new<F>(fn_once: F) -> Self
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
    /// Calls the stored `FnOnce`, consuming the erased thunk in the process.
    pub(crate) fn call(self) -> T {
        let this = ManuallyDrop::new(self);

        // SAFETY: `this` will not be dropped, so moving `inner` out cannot cause `Drop`
        // to run after the closure has been taken from the slot.
        let Inner {
            slot, call_impl, ..
        } = unsafe { read(&this.inner) };

        call_impl(slot)
    }
}

impl<T> Drop for ErasedFnOnce<'_, T> {
    fn drop(&mut self) {
        // SAFETY: We own the slot, and it cannot be used after dropping.
        unsafe { (self.inner.drop_in_place_impl)(&mut self.inner.slot) }
    }
}

impl<T> fmt::Debug for ErasedFnOnce<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ErasedFnOnce -> {}", type_name::<T>())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::ErasedFnOnce;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn sanity() {
        let thunk = ErasedFnOnce::new(|| 42);
        assert_eq!(42, thunk.call());
    }

    #[test]
    fn with_captures() {
        let x = 1;
        let y = 2;

        let thunk = ErasedFnOnce::new(move || x + y);

        assert_eq!(3, thunk.call());
    }

    #[test]
    #[should_panic]
    fn with_too_many_captures() {
        let a: u64 = 1;
        let b: u64 = 2;
        let c: u64 = 3;
        let d: u64 = 4;
        let e: u64 = 5;
        let f: u64 = 6;
        let g: u64 = 7;
        let h: u64 = 8;

        let _ = ErasedFnOnce::new(move || a + b + c + d + e + f + g + h);
    }

    #[test]
    fn dropping_without_call_runs_destructor_once() {
        let drops = std::rc::Rc::new(std::cell::Cell::new(0));
        let tracker = DropTracker {
            drops: std::rc::Rc::clone(&drops),
        };
        let thunk = ErasedFnOnce::new(move || {
            let _tracker = tracker;
        });

        drop(thunk);

        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn calling_runs_destructor_once() {
        let drops = std::rc::Rc::new(std::cell::Cell::new(0));
        let tracker = DropTracker {
            drops: std::rc::Rc::clone(&drops),
        };
        let thunk = ErasedFnOnce::new(move || {
            let _tracker = tracker;
        });

        thunk.call();

        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn panic_during_call_drops_capture_once() {
        let drops = std::rc::Rc::new(std::cell::Cell::new(0));
        let tracker = DropTracker {
            drops: std::rc::Rc::clone(&drops),
        };
        let thunk = ErasedFnOnce::new(move || {
            let _tracker = tracker;
            panic!("boom");
        });

        let _ = catch_unwind(AssertUnwindSafe(|| thunk.call()));

        assert_eq!(drops.get(), 1);
    }

    struct DropTracker {
        drops: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }
}
