use core::{
    marker::PhantomData,
    mem::{align_of, forget, size_of, MaybeUninit},
    ptr::{drop_in_place, NonNull},
};

const SLOT_SIZE: usize = 128;

#[repr(C, align(128))]
pub struct Slot {
    bytes: MaybeUninit<[u8; SLOT_SIZE]>,
}

impl Default for Slot {
    fn default() -> Self {
        Self::new()
    }
}

impl Slot {
    pub const fn new() -> Self {
        Self {
            bytes: MaybeUninit::uninit(),
        }
    }

    #[inline(always)]
    pub fn cast<T>(&mut self) -> &mut MaybeUninit<T> {
        let slot_ptr: *mut _ = self;

        // Verify the size and alignment of T.
        assert!(size_of::<T>() <= size_of::<Self>());
        assert!(align_of::<T>() <= align_of::<Self>());

        // SAFETY: We just checked the size and alignment of `T`. Since we are
        // only returning `MaybeUninit<T>`, we need not worry about the value.
        let casted = unsafe { &mut *self.bytes.as_mut_ptr().cast() };
        let casted_ptr: *mut _ = casted;

        // Verify that the address of the pointer has not actually changed. This
        // ensures that it is safe to recover the slot pointer from the value
        // pointer.
        assert_eq!(casted_ptr as usize, slot_ptr as usize);

        casted
    }
}

pub struct SlotBox<'slot, T: ?Sized + 'slot> {
    pointer: NonNull<T>,
    _marker: PhantomData<(&'slot mut Slot, T)>,
}

impl<'slot, T: ?Sized> SlotBox<'slot, T> {
    /// # Safety
    ///
    /// The caller must ensure that `value` is stored in a `Slot`.
    pub unsafe fn adopt(value: &'slot mut T) -> Self {
        Self {
            pointer: value.into(),
            _marker: PhantomData,
        }
    }

    pub fn coerce<U, F>(slot_box: Self, coerce_fn: F) -> SlotBox<'slot, U>
    where
        U: ?Sized,
        F: FnOnce(&mut T) -> &mut U,
    {
        let leaked = Self::leak(slot_box);
        let leaked_ptr: *mut _ = leaked;

        let coerced = coerce_fn(leaked);
        let coerced_ptr: *mut _ = coerced;

        assert_eq!(
            leaked_ptr as *mut u8 as usize,
            coerced_ptr as *mut u8 as usize,
        );

        // SAFETY: Since the addresss of the pointer did not change, we know
        // that the value is still in a slot and only the type has changed.
        unsafe { SlotBox::adopt(coerced) }
    }

    pub fn leak(slot_box: Self) -> &'slot mut T {
        let value_ptr = slot_box.pointer.as_ptr();
        forget(slot_box);

        // SAFETY: We know that the value is in the `Slot` because we placed it
        // there in `SlotBox::new_in`. Since the value cannot otherwise be
        // dropped, the reference is valid for the lifetime of the `Slot`.
        unsafe { &mut *value_ptr }
    }

    fn leak_as_slot(slot_box: Self) -> &'slot mut Slot {
        let slot_ptr: *mut Slot = slot_box.pointer.as_ptr().cast();
        forget(slot_box);

        // SAFETY: We checked in `Slot::cast` that the address of the value is
        // also the address of the slot.
        unsafe { &mut *slot_ptr }
    }
}

impl<'slot, T> SlotBox<'slot, T> {
    pub fn new_in(slot: &'slot mut Slot, value: T) -> Self {
        let value_ptr = slot.cast().write(value);

        Self {
            pointer: value_ptr.into(),
            _marker: PhantomData,
        }
    }

    pub fn unwrap(slot_box: Self) -> (&'slot mut Slot, T) {
        let slot = Self::leak_as_slot(slot_box);

        // SAFETY: We know there is a `T` in the `Slot` because we placed it
        // there in `SlotBox::new_in`.
        let value: T = unsafe { slot.cast().assume_init_read() };

        (slot, value)
    }
}

impl<T: ?Sized> Drop for SlotBox<'_, T> {
    fn drop(&mut self) {
        let value_ptr = self.pointer.as_ptr();

        // SAFETY: The `SlotBox` logically owns this pointer.
        unsafe { drop_in_place(value_ptr) }
    }
}
