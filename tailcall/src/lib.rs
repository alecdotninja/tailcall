//! Stack-safe tail calls on stable Rust.
//!
//! `tailcall` provides two layers:
//!
//! - the [`tailcall`] attribute macro, which rewrites a function either into an inline loop or to
//!   execute through the trampoline runtime
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
//! fn skip_leading_separators(rest: &[u8]) -> usize {
//!     match rest {
//!         [b' ' | b',', tail @ ..] => {
//!             tailcall::call! { skip_leading_separators(tail) }
//!         }
//!         _ => rest.len(),
//!     }
//! }
//!
//! assert_eq!(skip_leading_separators(b"  ,abc"), 3);
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
//!     fn is_even(&self, x: u32) -> bool {
//!         if x == 0 {
//!             true
//!         } else {
//!             tailcall::call! { self.is_odd(x - 1) }
//!         }
//!     }
//!
//!     #[tailcall]
//!     fn is_odd(&self, x: u32) -> bool {
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
//! [`call!`] is handled by the tailcall transform, while a plain recursive call remains an
//! ordinary Rust call:
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
//! When a `#[tailcall]` free function or inherent method only tail-calls itself directly, the
//! macro can lower it to an inline `loop`, which removes the trampoline overhead entirely. More
//! complex cases, such as mutual recursion or functions that need the full hidden builder shape,
//! still use the [`Thunk`]-based runtime.
//!
//! For methods, the optimized path works by aliasing the receiver once, rebinding the
//! non-receiver arguments as mutable loop state, and turning each direct self tail call into
//! "compute next arguments, assign them, and `continue`".
//!
//! ## Manual `Thunk`
//!
//! If you need direct control over the runtime, the low-level API is [`runtime::Thunk`]. It is
//! also re-exported at the crate root as [`Thunk`].
//! A [`Thunk`] is a fixed-size deferred value from a computation, which means it can live on the
//! stack. It may hold either the value directly or a type-erased closure that will eventually
//! produce the value.
//!
//! On 64-bit targets, the current runtime keeps [`Thunk`] at 32 bytes. It achieves that by using
//! a small inline storage slot for deferred closures, which means manual [`Thunk`] values and
//! macro-generated helpers can only capture a limited amount of data before construction panics.
//! Pending [`Thunk`] values still preserve normal destructor-on-drop behavior for their captures.
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
//! fn skip_leading_separators(input: &str) -> usize {
//!     __tailcall_build_skip_separators_thunk(input.as_bytes()).call()
//! }
//!
//! fn __tailcall_build_skip_separators_thunk<'a>(rest: &'a [u8]) -> Thunk<'a, usize> {
//!     Thunk::bounce(move || match rest {
//!         [b' ' | b',', tail @ ..] => __tailcall_build_skip_separators_thunk(tail),
//!         _ => Thunk::value(rest.len()),
//!     })
//! }
//!
//! assert_eq!(skip_leading_separators("  ,abc"), 3);
//! ```
//!
//! The primary limitation of [`Thunk`] is that it type-erases the deferred closure into a fixed
//! inline slot. That means each deferred closure can capture at most about 16 bytes of data on
//! the current implementation. If the closure's captures are larger than that, construction will
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
//! - mixed recursion is allowed, but only `tailcall::call!` sites participate in the tailcall
//!   transform; plain recursive calls still use the native call stack
//! - each generated helper is backed by a [`Thunk`], so very large argument lists or captures can
//!   exceed the 16-byte deferred-closure budget
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
pub use tailcall_proc_macro::{call, tailcall};

pub mod runtime;
