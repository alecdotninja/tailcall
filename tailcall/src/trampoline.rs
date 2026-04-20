//! A stack-reusing trampoline runtime.
//!
//! The runtime repeatedly evaluates [`crate::trampoline::Action`] values until a terminal
//! [`crate::trampoline::Action::Done`] is reached.
//! [`crate::trampoline::Action::Call`] stores the next step as a [`crate::thunk::Thunk`],
//! allowing each recursive step to reuse the same outer stack frame instead of recursing through
//! Rust's native call stack.

use crate::thunk::Thunk;

/// The next step to be executed by the trampoline runtime.
pub enum Action<'a, T> {
    /// Finish the computation and return the contained value.
    Done(T),
    /// Execute the next thunk and continue the trampoline loop.
    Call(Thunk<'a, Self>),
}

/// Produces a completed [`Action`].
pub const fn done<'a, T>(value: T) -> Action<'a, T> {
    Action::Done(value)
}

/// Produces a pending [`Action`] from a `FnOnce`.
///
/// The closure is executed later by [`run`].
pub const fn call<'a, T, F>(fn_once: F) -> Action<'a, T>
where
    F: FnOnce() -> Action<'a, T> + 'a,
{
    Action::Call(Thunk::new(fn_once))
}

/// Runs trampoline actions until they resolve to a final value.
pub fn run<T>(mut action: Action<'_, T>) -> T {
    loop {
        match action {
            Action::Call(thunk) => action = thunk.call(),
            Action::Done(value) => return value,
        }
    }
}
