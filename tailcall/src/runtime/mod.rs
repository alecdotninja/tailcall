//! Low-level runtime support for stack-safe tail calls.
//!
//! Most users should prefer the [`crate::tailcall`] macro, but the runtime can also be used
//! directly when more explicit control is useful.
//!
//! The main entry point is [`Thunk`], a fixed-size deferred value from a computation.
//! A [`Thunk`] may hold either the value directly or a type-erased closure that will eventually
//! produce the value.
//!
//! On 64-bit targets, the default runtime keeps [`Thunk`] at 32 bytes. Optional crate features
//! can opt into larger [`Thunk`] sizes to support larger inline captures. If a closure still
//! exceeds the configured inline budget, [`Thunk`] construction panics.
//!
//! Pending [`Thunk`] values still preserve normal destructor-on-drop behavior for anything they
//! capture.
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
//! fn skip_leading_separators(input: &str) -> usize {
//!     build_skip_separators(input.as_bytes()).call()
//! }
//!
//! fn build_skip_separators(rest: &[u8]) -> Thunk<'_, usize> {
//!     Thunk::bounce(move || match rest {
//!         [b' ' | b',', tail @ ..] => build_skip_separators(tail),
//!         _ => Thunk::value(rest.len()),
//!     })
//! }
//!
//! assert_eq!(skip_leading_separators("  ,abc"), 3);
//! ```

mod erased_fn_once;
mod slot;
mod thunk;

use erased_fn_once::ErasedFnOnce;
pub use thunk::Thunk;
