use crate::slot::Slot;
use core::{marker::PhantomData, mem::transmute, ptr::drop_in_place};

const MAX_THUNK_DATA_SIZE: usize = 48;

type ThunkSlot = Slot<MAX_THUNK_DATA_SIZE>;
type CallFn<T> = fn(ThunkSlot) -> T;
type DropInPlaceFn = unsafe fn(*mut ThunkSlot);

#[repr(transparent)]
pub struct Thunk<'a, T = ()> {
    inner: ThunkInner<'a, T>,
}

struct ThunkInner<'a, T> {
    slot: ThunkSlot,
    call_fn: CallFn<T>,
    drop_in_place_fn: DropInPlaceFn,
    _marker: PhantomData<dyn FnOnce() -> T + 'a>,
}

impl<'a, T> Thunk<'a, T> {
    pub const fn new<F>(fn_once: F) -> Self
    where
        F: FnOnce() -> T + 'a,
    {
        Self {
            inner: ThunkInner::new(fn_once),
        }
    }

    #[inline(always)]
    pub fn call(self) -> T {
        self.into_inner().call()
    }

    const fn into_inner(self) -> ThunkInner<'a, T> {
        // SAFETY: `Thunk` is a transparent wrapper around `ThunkInner`.
        unsafe { transmute(self) }
    }
}

impl<'a, T> Drop for Thunk<'a, T> {
    fn drop(&mut self) {
        // SAFETY: We own `inner`, and it cannot be used after dropping.
        unsafe { self.inner.drop_in_place() }
    }
}

impl<'a, T> ThunkInner<'a, T> {
    pub const fn new<F>(fn_once: F) -> Self
    where
        F: FnOnce() -> T + 'a,
    {
        Self {
            slot: Slot::new(fn_once),
            call_fn: |slot| {
                // SAFETY: `slot` is initialized above with `F`.
                unsafe { slot.into_value::<F>()() }
            },
            drop_in_place_fn: |slot_ptr| {
                // SAFETY: `slot` is initialized above with `F`.
                unsafe { drop_in_place(slot_ptr.cast::<F>()) };
            },
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn call(self) -> T {
        let Self { slot, call_fn, .. } = self;

        call_fn(slot)
    }

    // SAFETY: `Self::call` cannot be called after dropping in place.
    #[inline(always)]
    pub unsafe fn drop_in_place(&mut self) {
        unsafe { (self.drop_in_place_fn)(&mut self.slot) }
    }
}
