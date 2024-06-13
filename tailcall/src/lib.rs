#![no_std]
#![deny(
    trivial_casts,
    trivial_numeric_casts,
    unsafe_op_in_unsafe_fn,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

pub use tailcall_impl::tailcall;

pub mod thunk;
pub mod trampoline;
