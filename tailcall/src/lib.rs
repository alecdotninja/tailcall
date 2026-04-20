//! Stack-safe tail calls on stable Rust.
//!
//! `tailcall` provides two layers:
//!
//! - the [`tailcall`] attribute macro, which rewrites a function to execute through the
//!   trampoline runtime
//! - the runtime itself, exposed through [`trampoline`] and [`thunk`]
//!
//! The macro-based API is explicit at recursive call sites. Any tail call that should be executed
//! through the trampoline must use [`call!`]:
//!
//! ```rust
//! use tailcall::tailcall;
//!
//! #[tailcall]
//! fn gcd(a: u64, b: u64) -> u64 {
//!     if b == 0 {
//!         a
//!     } else {
//!         tailcall::call! { gcd(b, a % b) }
//!     }
//! }
//!
//! assert_eq!(gcd(12, 18), 6);
//! ```
//!
//! More complex stateful traversals can still use the macro directly:
//!
//! ```rust
//! use tailcall::tailcall;
//!
//! #[tailcall]
//! fn sum_csv_numbers(rest: &[u8], total: u64, current: u64) -> u64 {
//!     match rest {
//!         [digit @ b'0'..=b'9', tail @ ..] => {
//!             let current = current * 10 + u64::from(digit - b'0');
//!             tailcall::call! { sum_csv_numbers(tail, total, current) }
//!         }
//!         [b' ' | b',', tail @ ..] => {
//!             let total = total + current;
//!             tailcall::call! { sum_csv_numbers(tail, total, 0) }
//!         }
//!         [] => total + current,
//!         [_other, tail @ ..] => {
//!             tailcall::call! { sum_csv_numbers(tail, total, current) }
//!         }
//!     }
//! }
//!
//! assert_eq!(sum_csv_numbers(b"10, 20, 3", 0, 0), 33);
//! ```
//!
//! Mutual recursion also works through the macro API as long as each participating function is
//! annotated with [`tailcall`] and each tail-call site uses [`call!`]:
//!
//! ```rust
//! use tailcall::tailcall;
//!
//! #[tailcall]
//! fn is_even(x: u128) -> bool {
//!     if x == 0 {
//!         true
//!     } else {
//!         tailcall::call! { is_odd(x - 1) }
//!     }
//! }
//!
//! #[tailcall]
//! fn is_odd(x: u128) -> bool {
//!     if x == 0 {
//!         false
//!     } else {
//!         tailcall::call! { is_even(x - 1) }
//!     }
//! }
//!
//! assert!(is_even(1000));
//! assert!(is_odd(1001));
//! ```
//!
//! Limitations of the current macro:
//!
//! - tail-call sites must be written as `tailcall::call! { path(args...) }`
//! - methods and `self` receivers are not supported
//! - argument patterns must be simple identifiers
//! - `?` is not supported inside `#[tailcall]` functions on stable Rust; use `match` or explicit
//!   early returns instead
//!
//! The runtime can also be used directly for advanced manual control, but most users should only
//! need the macro API shown above.
//!
#![no_std]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_op_in_unsafe_fn,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

pub use tailcall_impl::{call, tailcall};

pub(crate) mod slot;
/// Type-erased `FnOnce` storage used by the trampoline runtime.
pub mod thunk;
/// The stack-reusing trampoline runtime used by the public macro.
pub mod trampoline;
