// Private implementation details for the public `crate::runtime::Thunk` type.

use core::{any::type_name, fmt};

use super::ErasedFnOnce;

/// A fixed-size deferred value in the thunk runtime.
///
/// A [`Thunk`] is small enough to live on the stack. It may hold either the value directly or a
/// type-erased closure that will eventually produce the value.
///
/// On 64-bit targets, the current runtime representation keeps [`Thunk`] at 32 bytes. That size
/// target is achieved by storing deferred closures in a small inline slot. As a result, manual
/// [`Thunk`] values and macro-generated helpers can only capture a limited amount of data before
/// construction panics.
///
/// Dropping a pending [`Thunk`] still preserves normal destructor semantics for its captured
/// values.
///
/// Values of this type are created with [`Thunk::new`], [`Thunk::value`], and [`Thunk::bounce`],
/// then consumed by [`Thunk::call`].
pub struct Thunk<'a, T>(ThunkKind<'a, T>);

enum ThunkKind<'a, T> {
    Done(T),
    Bounce(ErasedFnOnce<'a, Thunk<'a, T>>),
}

impl<'a, T> Thunk<'a, T> {
    /// Produces a pending [`Thunk`] from a `FnOnce` that resolves directly to a value.
    pub const fn new<F>(fn_once: F) -> Self
    where
        F: FnOnce() -> T + 'a,
    {
        Self::bounce(move || Self::value(fn_once()))
    }

    /// Produces a [`Thunk`] that resolves directly to a value.
    pub const fn value(value: T) -> Self {
        Self(ThunkKind::Done(value))
    }

    /// Produces a pending [`Thunk`] from a `FnOnce`.
    ///
    /// The closure must return the next [`Thunk`] in the computation.
    pub const fn bounce<F>(fn_once: F) -> Self
    where
        F: FnOnce() -> Self + 'a,
    {
        Self(ThunkKind::Bounce(ErasedFnOnce::new(fn_once)))
    }

    /// Resolves the deferred computation to a final value.
    #[inline(always)]
    pub fn call(mut self) -> T {
        loop {
            match self.0 {
                ThunkKind::Bounce(erased_fn_once) => self = erased_fn_once.call(),
                ThunkKind::Done(value) => return value,
            }
        }
    }
}

impl<T> fmt::Debug for Thunk<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Thunk -> {}", type_name::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::Thunk;
    use core::mem::size_of;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn thunk_is_32_bytes_on_64_bit_targets() {
        assert_eq!(size_of::<Thunk<'static, ()>>(), 32);
        assert_eq!(size_of::<Thunk<'static, bool>>(), 32);
        assert_eq!(size_of::<Thunk<'static, u64>>(), 32);
    }
}
