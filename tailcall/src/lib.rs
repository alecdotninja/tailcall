#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

pub use tailcall_impl::tailcall;

pub mod slot;
pub mod thunk;
pub mod trampoline;
