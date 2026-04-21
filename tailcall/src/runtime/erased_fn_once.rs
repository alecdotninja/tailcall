//! Type-erased `FnOnce` storage for the thunk runtime.
//!
//! An [`ErasedFnOnce`] stores the captured data for a single `FnOnce` in a fixed-size stack slot
//! together with the function pointers needed to either call it or drop it in place.

use core::{
    any::type_name,
    fmt,
    marker::PhantomData,
    mem::ManuallyDrop,
    ptr::{drop_in_place, read, NonNull},
};

use super::slot::Slot;

// On 64-bit targets, 16 bytes of inline capture storage is the smallest useful budget with the
// runtime's 16-byte slot alignment. Combined with the non-null shared vtable pointer below, that
// allows the public `Thunk` representation to fit in 32 bytes while preserving
// destructor-on-drop semantics.
const MAX_CLOSURE_DATA_SIZE: usize = 16;

type ErasedFnOnceSlot = Slot<MAX_CLOSURE_DATA_SIZE>;
type CallFn<T> = unsafe fn(ErasedFnOnceSlot) -> T;
type DropInPlaceFn = unsafe fn(*mut ErasedFnOnceSlot);

struct ErasedFnOnceVtable<T> {
    call_impl: CallFn<T>,
    drop_in_place_impl: DropInPlaceFn,
}

pub(crate) struct ErasedFnOnce<'a, T = ()> {
    slot: ErasedFnOnceSlot,
    vtable: NonNull<ErasedFnOnceVtable<T>>,
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
            slot: Slot::new(fn_once),
            vtable: {
                let vtable: *const ErasedFnOnceVtable<T> = &ErasedFnOnceVtable {
                    call_impl: |slot| {
                        // SAFETY: `slot` is initialized above with `F`.
                        unsafe { slot.into_value::<F>()() }
                    },
                    drop_in_place_impl: |slot_ptr| {
                        // SAFETY: `slot` is initialized above with `F`.
                        unsafe { drop_in_place(slot_ptr.cast::<F>()) };
                    },
                };

                // SAFETY: `vtable` points at the static per-closure-type table above and is
                // therefore never null.
                unsafe { NonNull::new_unchecked(vtable.cast_mut()) }
            },
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    fn vtable(&self) -> &ErasedFnOnceVtable<T> {
        // SAFETY: `vtable` always points at the static per-closure-type table created in `new`.
        unsafe { self.vtable.as_ref() }
    }

    #[inline(always)]
    /// Calls the stored `FnOnce`, consuming the erased thunk in the process.
    pub(crate) fn call(self) -> T {
        let this = ManuallyDrop::new(self);

        // SAFETY: `this` will not be dropped, so moving the stored slot and vtable pointer out
        // cannot cause the destructor path to run after the closure has been taken from the slot.
        let slot = unsafe { read(&this.slot) };
        let vtable = this.vtable();

        // SAFETY: This is the exact `call_impl` for the slot created above.
        unsafe { (vtable.call_impl)(slot) }
    }
}

impl<T> Drop for ErasedFnOnce<'_, T> {
    fn drop(&mut self) {
        // SAFETY: We own the slot, and it cannot be used after dropping.
        unsafe { (self.vtable().drop_in_place_impl)(&mut self.slot) }
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
