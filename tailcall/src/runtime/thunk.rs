//! A stack-reusing thunk runtime.
//!
//! This module is the low-level runtime behind the [`crate::tailcall`] macro.
//! Most users should prefer the macro API, but the runtime can also be used directly when more
//! explicit control is useful.
//!
//! The model is simple:
//!
//! - [`Thunk`](crate::Thunk) represents a deferred value from a computation
//! - [`Thunk::value`](crate::Thunk::value) wraps a value directly
//! - [`Thunk::new`](crate::Thunk::new) wraps a closure that will produce the value
//! - [`Thunk::bounce`](crate::Thunk::bounce) wraps a closure that will produce another
//!   [`Thunk`](crate::Thunk)
//! - [`Thunk::call`](crate::Thunk::call) resolves the whole computation to its final value
//!
//! A direct runtime implementation usually consists of:
//!
//! 1. an entry-point function that calls [`Thunk::call`](crate::Thunk::call)
//! 2. one or more builder functions that return [`Thunk`](crate::Thunk)
//! 3. recursive transitions expressed by returning another builder's
//!    [`Thunk`](crate::Thunk)
//!
//! For example:
//!
//! ```rust
//! use tailcall::Thunk;
//!
//! fn is_even(x: u128) -> bool {
//!     build_is_even_action(x).call()
//! }
//!
//! fn build_is_even_action(x: u128) -> Thunk<'static, bool> {
//!     Thunk::bounce(move || {
//!         if x == 0 {
//!             Thunk::value(true)
//!         } else {
//!             build_is_odd_action(x - 1)
//!         }
//!     })
//! }
//!
//! fn build_is_odd_action(x: u128) -> Thunk<'static, bool> {
//!     Thunk::bounce(move || {
//!         if x == 0 {
//!             Thunk::value(false)
//!         } else {
//!             build_is_even_action(x - 1)
//!         }
//!     })
//! }
//!
//! assert!(is_even(1000));
//! ```
//!
//! The lifetime parameter on [`Thunk`](crate::Thunk) is the lifetime of the values captured by the
//! deferred closure. For borrowed input, thread that lifetime through the builder functions and
//! finish the computation with [`Thunk::call`](crate::Thunk::call):
//!
//! ```rust
//! use tailcall::Thunk;
//!
//! fn sum_csv(input: &str) -> u64 {
//!     build_skip_separators(input.as_bytes(), 0).call()
//! }
//!
//! fn build_skip_separators<'a>(rest: &'a [u8], total: u64) -> Thunk<'a, u64> {
//!     Thunk::bounce(move || match rest {
//!         [b' ' | b',', tail @ ..] => build_skip_separators(tail, total),
//!         [] => Thunk::value(total),
//!         _ => build_read_number(rest, total, 0),
//!     })
//! }
//!
//! fn build_read_number<'a>(
//!     rest: &'a [u8],
//!     total: u64,
//!     current: u64,
//! ) -> Thunk<'a, u64> {
//!     Thunk::bounce(move || match rest {
//!         [digit @ b'0'..=b'9', tail @ ..] => {
//!             let current = current * 10 + u64::from(digit - b'0');
//!             build_read_number(tail, total, current)
//!         }
//!         _ => build_skip_separators(rest, total + current),
//!     })
//! }
//!
//! assert_eq!(sum_csv("10, 20, 3"), 33);
//! ```

use core::{any::type_name, fmt};

use super::ErasedThunk;

/// An opaque deferred value in the thunk runtime.
///
/// Values of this type are usually created with [`Thunk::bounce`], [`Thunk::new`], and
/// [`Thunk::value`] and then consumed by [`Thunk::call`].
/// Users do not inspect or construct the internal representation directly.
pub struct Thunk<'a, T>(ThunkKind<'a, T>);

enum ThunkKind<'a, T> {
    Done(T),
    Bounce(ErasedThunk<'a, Thunk<'a, T>>),
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
        Self(ThunkKind::Bounce(ErasedThunk::new(fn_once)))
    }

    /// Resolves the deferred computation to a final value.
    pub fn call(mut self) -> T {
        loop {
            match self.0 {
                ThunkKind::Bounce(thunk) => self = thunk.call(),
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
