#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! Tailcall is a library that adds safe, zero-cost [tail recursion] to stable Rust.
//! Eventually, it will be superseded by the [`become` keyword].
//!
//! # Usage
//!
//! To guarantee that recursive calls a function will reuse the same stack frame,
//! annotate it with the [`tailcall`] attribute.
//!
//! ```
//! use tailcall::tailcall;
//!
//! fn factorial(input: u64) -> u64 {
//!     #[tailcall]
//!     fn factorial_inner(accumulator: u64, input: u64) -> u64 {
//!         if input > 0 {
//!             factorial_inner(accumulator * input, input - 1)
//!         } else {
//!             accumulator
//!         }
//!     }
//!
//!     factorial_inner(1, input)
//! }
//! ```
//!
//! Recursive calls which are not in tail form will result in a compile-time error.
//!
//! ```compile_fail
//! use tailcall::tailcall;
//!   
//! #[tailcall]
//! fn factorial(input: u64) -> u64 {
//!     if input > 0 {
//!         input * factorial(input - 1)
//!     } else {
//!         1
//!     }
//! }
//! ```
//!
//! [tail recursion]: https://en.wikipedia.org/wiki/Tail_call
//! [`become` keyword]: https://internals.rust-lang.org/t/pre-rfc-explicit-proper-tail-calls/3797/16
//! [`tailcall`]: attr.tailcall.html

pub use tailcall_impl::tailcall;
pub use tailcall_impl::tailcall_opt;
pub use tailcall_impl::tailcall_res;

pub mod trampoline;
