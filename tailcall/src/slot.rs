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
    pub(crate) const fn new<T>(value: T) -> Self {
        assert!(
            align_of::<T>() <= align_of::<Self>(),
            "unsupport value alignment",
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
