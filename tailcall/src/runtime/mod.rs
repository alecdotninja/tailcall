//! Low-level runtime support for stack-safe tail calls.
//!
//! Most users should prefer the [`crate::tailcall`] macro, but the runtime can also be used
//! directly when more explicit control is useful.
//!
//! The main entry point is [`Thunk`], a fixed-size deferred value from a computation.
//! A [`Thunk`] may hold either the value directly or a type-erased closure that will eventually
//! produce the value.
//!
//! You can construct one in three ways:
//!
//! - [`Thunk::value`] wraps a value directly
//! - [`Thunk::new`] wraps a closure that will produce the value
//! - [`Thunk::bounce`] wraps a closure that will produce another [`Thunk`]
//!
//! The full computation is resolved with [`Thunk::call`].
//!
//! A direct runtime implementation usually consists of:
//!
//! 1. an entry-point function that calls [`Thunk::call`]
//! 2. one or more builder functions that return [`Thunk`]
//! 3. recursive transitions expressed by returning another builder's [`Thunk`]
//!
//! ```rust
//! use tailcall::runtime::Thunk;
//!
//! fn is_even(x: u128) -> bool {
//!     build_is_even(x).call()
//! }
//!
//! fn build_is_even(x: u128) -> Thunk<'static, bool> {
//!     Thunk::bounce(move || {
//!         if x == 0 {
//!             Thunk::value(true)
//!         } else {
//!             build_is_odd(x - 1)
//!         }
//!     })
//! }
//!
//! fn build_is_odd(x: u128) -> Thunk<'static, bool> {
//!     Thunk::bounce(move || {
//!         if x == 0 {
//!             Thunk::value(false)
//!         } else {
//!             build_is_even(x - 1)
//!         }
//!     })
//! }
//!
//! assert!(is_even(1000));
//! ```
//!
//! For borrowed input, thread the capture lifetime through your builder functions:
//!
//! ```rust
//! use tailcall::runtime::Thunk;
//!
//! fn sum_csv(input: &str) -> u64 {
//!     build_skip_separators(input.as_bytes(), 0).call()
//! }
//!
//! fn build_skip_separators(rest: &[u8], total: u64) -> Thunk<'_, u64> {
//!     Thunk::bounce(move || match rest {
//!         [b' ' | b',', tail @ ..] => build_skip_separators(tail, total),
//!         [] => Thunk::value(total),
//!         _ => build_read_number(rest, total, 0),
//!     })
//! }
//!
//! fn build_read_number(rest: &[u8], total: u64, current: u64) -> Thunk<'_, u64> {
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

mod erased_fn_once;
mod slot;
mod thunk;

use erased_fn_once::ErasedFnOnce;
pub use thunk::Thunk;
