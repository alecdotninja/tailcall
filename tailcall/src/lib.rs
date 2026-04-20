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
//! Methods in `impl` blocks are also supported:
//!
//! ```rust
//! use tailcall::tailcall;
//!
//! struct Parity;
//!
//! impl Parity {
//!     #[tailcall]
//!     fn is_even(&self, x: u128) -> bool {
//!         if x == 0 {
//!             true
//!         } else {
//!             tailcall::call! { self.is_odd(x - 1) }
//!         }
//!     }
//!
//!     #[tailcall]
//!     fn is_odd(&self, x: u128) -> bool {
//!         if x == 0 {
//!             false
//!         } else {
//!             tailcall::call! { self.is_even(x - 1) }
//!         }
//!     }
//! }
//!
//! let parity = Parity;
//! assert!(parity.is_even(1000));
//! ```
//!
//! Mixed recursion is also allowed within a `#[tailcall]` function. A recursive call written with
//! [`call!`] is trampoline-backed, while a plain recursive call remains an ordinary Rust call:
//!
//! ```rust
//! use tailcall::tailcall;
//!
//! #[tailcall]
//! fn mixed_recursion_sum(n: u64) -> u64 {
//!     match n {
//!         0 => 0,
//!         1 => tailcall::call! { mixed_recursion_sum(0) },
//!         _ if n % 2 == 0 => {
//!             let partial = mixed_recursion_sum(n - 1);
//!             n + partial
//!         }
//!         _ => tailcall::call! { mixed_recursion_sum(n - 1) },
//!     }
//! }
//!
//! assert_eq!(mixed_recursion_sum(6), 12);
//! ```
//!
//! If only part of a larger algorithm is tail-recursive, it can still be cleaner to annotate a
//! helper that contains just the tail-recursive portion:
//!
//! ```rust
//! use tailcall::tailcall;
//!
//! fn factorial(n: u64) -> u64 {
//!     #[tailcall]
//!     fn factorial_inner(acc: u64, n: u64) -> u64 {
//!         if n == 0 {
//!             acc
//!         } else {
//!             tailcall::call! { factorial_inner(acc * n, n - 1) }
//!         }
//!     }
//!
//!     factorial_inner(1, n)
//! }
//!
//! fn weighted_countdown(n: u64) -> u64 {
//!     if n <= 3 {
//!         n + factorial(n)
//!     } else {
//!         factorial(n / 2)
//!     }
//! }
//!
//! assert_eq!(weighted_countdown(3), 9);
//! assert_eq!(weighted_countdown(8), 24);
//! ```
//!
//! Limitations of the current macro:
//!
//! - tail-call sites must be written as `tailcall::call! { path(args...) }` or
//!   `tailcall::call! { self.method(args...) }`
//! - argument patterns must be simple identifiers
//! - `?` is not supported inside `#[tailcall]` functions on stable Rust; use `match` or explicit
//!   early returns instead
//! - trait methods are not supported yet
//! - mixed recursion is allowed, but only `tailcall::call!` sites are trampoline-backed; plain
//!   recursive calls still use the native call stack
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
