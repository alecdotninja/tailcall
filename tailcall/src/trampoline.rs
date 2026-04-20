//! A stack-reusing trampoline runtime.
//!
//! The runtime repeatedly evaluates [`crate::trampoline::Action`] values until they resolve to a
//! final value. Each pending step stores the next call as an internal thunk, allowing each
//! recursive step to reuse the same outer stack frame instead of recursing through Rust's native
//! call stack.

use crate::thunk::Thunk;

/// An opaque step in the trampoline runtime.
pub struct Action<'a, T>(ActionKind<'a, T>);

enum ActionKind<'a, T> {
    Done(T),
    Call(Thunk<'a, Action<'a, T>>),
}

/// Produces a completed [`Action`].
pub const fn done<'a, T>(value: T) -> Action<'a, T> {
    Action(ActionKind::Done(value))
}

/// Produces a pending [`Action`] from a `FnOnce`.
///
/// The closure is executed later by [`run`].
pub const fn call<'a, T, F>(fn_once: F) -> Action<'a, T>
where
    F: FnOnce() -> Action<'a, T> + 'a,
{
    Action(ActionKind::Call(Thunk::new(fn_once)))
}

/// Runs trampoline actions until they resolve to a final value.
pub fn run<T>(mut action: Action<'_, T>) -> T {
    loop {
        match action.0 {
            ActionKind::Call(thunk) => action = thunk.call(),
            ActionKind::Done(value) => return value,
        }
    }
}
