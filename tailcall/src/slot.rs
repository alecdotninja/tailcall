use core::mem::{align_of, size_of, MaybeUninit};

#[repr(C, align(128))]
pub struct Slot<const SIZE: usize = 128> {
    bytes: MaybeUninit<[u8; SIZE]>,
}

impl<const SIZE: usize> Slot<SIZE> {
    pub const fn new() -> Self {
        Self {
            bytes: MaybeUninit::uninit(),
        }
    }

    pub unsafe fn take<T>(in_slot: &mut T) -> (T, &mut Self) {
        let in_slot: *mut T = in_slot;
        debug_assert!((in_slot as usize) % align_of::<Self>() == 0);

        let slot: &mut Self = &mut *in_slot.cast();
        let value = slot.cast().assume_init_read();

        (value, slot)
    }

    pub fn put<T>(&mut self, value: T) -> &mut T {
        self.cast().write(value)
    }

    fn cast<T>(&mut self) -> &mut MaybeUninit<T> {
        debug_assert!(size_of::<T>() <= SIZE);
        debug_assert!(align_of::<T>() <= align_of::<Self>());

        // SAFETY: We just checked the size and alignment of T.
        unsafe { &mut *self.bytes.as_mut_ptr().cast() }
    }
}
