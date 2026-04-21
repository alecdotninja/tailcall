use core::mem::{align_of, size_of, ManuallyDrop, MaybeUninit};

#[repr(C, align(16))]
pub(crate) struct Slot<const SIZE: usize> {
    bytes: MaybeUninit<[u8; SIZE]>,
}

#[repr(C)]
union SlotView<T, const SIZE: usize> {
    value: ManuallyDrop<T>,
    slot: ManuallyDrop<Slot<SIZE>>,
}

impl<const SIZE: usize> Slot<SIZE> {
    // `Slot<SIZE>` can store any `T` that fits within the slot's full layout, including any tail
    // padding introduced by the alignment on `Slot` itself.
    pub(crate) const fn new<T>(value: T) -> Self {
        assert!(
            align_of::<T>() <= align_of::<Self>(),
            "unsupported value alignment",
        );

        assert!(
            size_of::<T>() <= size_of::<Self>(),
            "value size exceeds slot capacity",
        );

        SlotView::of_value(value).into_slot()
    }

    // SAFETY: The caller must ensure that `self` contains a valid `T`.
    pub(crate) const unsafe fn into_value<T>(self) -> T {
        unsafe { SlotView::of_slot(self).into_value() }
    }
}

impl<T, const SIZE: usize> SlotView<T, SIZE> {
    const fn of_value(value: T) -> Self {
        Self {
            value: ManuallyDrop::new(value),
        }
    }

    const fn of_slot(slot: Slot<SIZE>) -> Self {
        Self {
            slot: ManuallyDrop::new(slot),
        }
    }

    const fn into_slot(self) -> Slot<SIZE> {
        // SAFETY: `Slot<SIZE>` is valid at all bit patterns.
        ManuallyDrop::into_inner(unsafe { self.slot })
    }

    // SAFETY: The caller must ensure that `self` contains a valid `T`.
    const unsafe fn into_value(self) -> T {
        ManuallyDrop::into_inner(unsafe { self.value })
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::Slot;

    #[repr(align(32))]
    struct OverAligned;

    #[test]
    fn round_trips_stored_value() {
        let slot = Slot::<8>::new(42_u64);
        let value = unsafe { slot.into_value::<u64>() };

        assert_eq!(value, 42);
    }

    #[test]
    fn can_use_tail_padding_in_slot_layout() {
        let value = [7_u8; 16];
        let slot = Slot::<1>::new(value);
        let round_trip = unsafe { slot.into_value::<[u8; 16]>() };

        assert_eq!(round_trip, value);
    }

    #[test]
    #[should_panic(expected = "value size exceeds slot capacity")]
    fn rejects_values_that_exceed_slot_capacity() {
        let _ = Slot::<16>::new([0_u8; 17]);
    }

    #[test]
    #[should_panic(expected = "unsupported value alignment")]
    fn rejects_values_with_unsupported_alignment() {
        let _ = Slot::<32>::new(OverAligned);
    }
}
