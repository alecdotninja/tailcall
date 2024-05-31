use core::mem::{align_of, size_of, MaybeUninit};

#[repr(C, align(128))]
pub struct Slot<const SIZE: usize = 128> {
    bytes: MaybeUninit<[u8; SIZE]>,
}

impl<const SIZE: usize> Default for Slot<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const SIZE: usize> Slot<SIZE> {
    pub const fn new() -> Self {
        Self {
            bytes: MaybeUninit::uninit(),
        }
    }

    #[inline(always)]
    pub fn cast<T>(&mut self) -> &mut MaybeUninit<T> {
        let slot_ptr = self as *mut _;

        // Verify the size and alignment of T.
        assert!(size_of::<T>() <= SIZE);
        assert!(align_of::<T>() <= align_of::<Self>());

        // SAFETY: We just checked the size and alignment of T. Since we are
        // only returning `MaybeUninit<T>`, we need not worry about the bits.
        let casted = unsafe { &mut *self.bytes.as_mut_ptr().cast() };
        let casted_ptr = casted as *mut _;

        // Verify that the address of the pointer has not actually changed. This
        // ensures that it is safe to transume between `&mut Slot` and `&mut T`
        // (provided that there is a `T` in the slot).
        assert!(casted_ptr as usize == slot_ptr as usize);

        casted
    }
}
