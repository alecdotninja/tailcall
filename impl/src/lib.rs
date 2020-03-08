//! This crate contains the procedural macro implementation for the [tailcall] crate.
//! It is not designed to be used dierctly.
//! [tailcall]: https://crates.io/crates/tailcall

#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

extern crate proc_macro;

mod helpers;
mod transforms;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Transforms a [function definition] so that all recursive calls within the body are
/// guaranteed to use a single stack frame (via [tail recursion]).
///
/// [function definition]: https://docs.rs/syn/1.0.9/syn/struct.ItemFn.html
/// [tail recursion]: https://en.wikipedia.org/wiki/Tail_call
///
/// # Example
///
/// ```
/// use tailcall::tailcall;
///
/// fn factorial(input: u64) -> u64 {
///     #[tailcall]
///     fn factorial_inner(accumulator: u64, input: u64) -> u64 {
///         if input > 0 {
///             factorial_inner(accumulator * input, input - 1)
///         } else {
///             accumulator
///         }
///     }
///
///     factorial_inner(1, input)
/// }
/// ```
///
/// # Requirements
///
/// - All recursive calls must be in [tail form]:
///
/// ```compile_fail
/// use tailcall::tailcall;
///   
/// #[tailcall]
/// fn factorial(input: u64) -> u64 {
///     if input > 0 {
///         input * factorial(input - 1)
/// //      ^^^^^^^ This is not allowed.
///     } else {
///         1
///     }
/// }
/// ```
///
/// - Methods (functions which bind `self` in the arguments list) are not supported:
///
/// ```compile_fail
/// trait Factorialable {
///     fn factorial(self) -> Self {
///         self.calc_factorial(1)
///     }
///
///     fn calc_factorial(self, accumulator: u64) -> u64;
/// }
///
/// impl Factorialable for u64 {
///     #[tailcall]
///     fn calc_factorial(self, accumulator: u64) -> u64 {
/// //                    ^^^^ This is not allowed.
///         if self > 0 {
///             (self - 1).calc_factorial(self * accumulator)
///         } else {
///             accumulator
///         }
///     }
/// }
/// ```
///
/// [tail form]: https://en.wikipedia.org/wiki/Tail_call
#[proc_macro_attribute]
pub fn tailcall(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as ItemFn);
    let output = transforms::apply_fn_tailcall_transform(input);

    TokenStream::from(quote! {
        #output
    })
}
