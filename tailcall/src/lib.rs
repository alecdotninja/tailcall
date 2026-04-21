//! Stack-safe tail calls on stable Rust.
//!
//! `tailcall` provides two layers:
//!
//! - the [`tailcall`] attribute macro, which rewrites a function to execute through the
//!   trampoline runtime
//! - the low-level runtime, exposed as [`runtime`] and re-exported at the crate root as [`Thunk`]
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
//! In practice, most users should stop here. The macro handles the trampoline machinery and lets
//! you write recursive code directly, with [`call!`] marking the tail-recursive transitions.
//!
//! ## Manual `Thunk`
//!
//! If you need direct control over the runtime, the low-level API is [`runtime::Thunk`]. It is
//! also re-exported at the crate root as [`Thunk`].
//! A [`Thunk`] is a fixed-size deferred value from a computation, which means it can live on the
//! stack. It may hold either the value directly or a type-erased closure that will eventually
//! produce the value.
//!
//! You can construct one in three ways:
//!
//! - [`Thunk::value`] wraps a value directly
//! - [`Thunk::new`] wraps a closure that will produce the value
//! - [`Thunk::bounce`] wraps a closure that will produce another [`Thunk`], which will then
//!   provide the value
//!
//! The full computation is resolved with [`Thunk::call`].
//!
//! A manual runtime implementation usually looks like this:
//!
//! ```rust
//! use tailcall::runtime::Thunk;
//!
//! fn is_even(x: u128) -> bool {
//!     __tailcall_build_is_even_thunk(x).call()
//! }
//!
//! fn __tailcall_build_is_even_thunk(x: u128) -> Thunk<'static, bool> {
//!     Thunk::bounce(move || {
//!         if x == 0 {
//!             Thunk::value(true)
//!         } else {
//!             __tailcall_build_is_odd_thunk(x - 1)
//!         }
//!     })
//! }
//!
//! fn __tailcall_build_is_odd_thunk(x: u128) -> Thunk<'static, bool> {
//!     Thunk::bounce(move || {
//!         if x == 0 {
//!             Thunk::value(false)
//!         } else {
//!             __tailcall_build_is_even_thunk(x - 1)
//!         }
//!     })
//! }
//!
//! assert!(is_even(1000));
//! ```
//!
//! [`Thunk::new`] is a convenience for the common case where one deferred step immediately
//! resolves to a final value:
//!
//! ```rust
//! use tailcall::runtime::Thunk;
//!
//! fn answer() -> i32 {
//!     Thunk::new(|| 42).call()
//! }
//!
//! assert_eq!(answer(), 42);
//! ```
//!
//! Borrowed input works too. The lifetime on [`Thunk`] is the lifetime of the values captured by
//! the deferred closure:
//!
//! ```rust
//! use tailcall::runtime::Thunk;
//!
//! fn sum_csv(input: &str) -> u64 {
//!     __tailcall_build_skip_separators_thunk(input.as_bytes(), 0).call()
//! }
//!
//! fn __tailcall_build_skip_separators_thunk<'a>(rest: &'a [u8], total: u64) -> Thunk<'a, u64> {
//!     Thunk::bounce(move || match rest {
//!         [b' ' | b',', tail @ ..] => __tailcall_build_skip_separators_thunk(tail, total),
//!         [] => Thunk::value(total),
//!         _ => __tailcall_build_read_number_thunk(rest, total, 0),
//!     })
//! }
//!
//! fn __tailcall_build_read_number_thunk<'a>(rest: &'a [u8], total: u64, current: u64) -> Thunk<'a, u64> {
//!     Thunk::bounce(move || match rest {
//!         [digit @ b'0'..=b'9', tail @ ..] => {
//!             let current = current * 10 + u64::from(digit - b'0');
//!             __tailcall_build_read_number_thunk(tail, total, current)
//!         }
//!         _ => __tailcall_build_skip_separators_thunk(rest, total + current),
//!     })
//! }
//!
//! assert_eq!(sum_csv("10, 20, 3"), 33);
//! ```
//!
//! The primary limitation of [`Thunk`] is that it type-erases the deferred closure into a fixed
//! inline slot. That means each deferred closure can capture at most 48 bytes of data on the
//! current implementation. If the closure's captures are larger than that, construction will
//! panic.
//!
//! ## How The Macro Fits
//!
//! `#[tailcall]` generates the same kind of `Thunk`-returning helper that you would write by
//! hand and then calls [`Thunk::call`] in the public wrapper.
//!
//! At a high level, this:
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
//! ```
//!
//! behaves roughly like:
//!
//! ```rust
//! fn gcd(a: u64, b: u64) -> u64 {
//!     __tailcall_build_gcd_thunk(a, b).call()
//! }
//!
//! fn __tailcall_build_gcd_thunk<'tailcall>(a: u64, b: u64) -> tailcall::runtime::Thunk<'tailcall, u64> {
//!     tailcall::runtime::Thunk::bounce(move || {
//!         if b == 0 {
//!             tailcall::runtime::Thunk::value(a)
//!         } else {
//!             __tailcall_build_gcd_thunk(b, a % b)
//!         }
//!     })
//! }
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
//! - each generated helper is backed by a [`Thunk`], so very large argument lists or captures can
//!   exceed the 48-byte deferred-closure budget
//!
//! The runtime can also be used directly through [`Thunk`] when you want to build the state
//! machine yourself, but most users should only need the macro API shown above.
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

pub use runtime::Thunk;
pub use tailcall_impl::{call, tailcall};

pub mod runtime;
