//! A stack-reusing trampoline runtime.
//!
//! This module is the public low-level runtime behind the [`crate::tailcall`] macro.
//! Most users should prefer the macro API, but the runtime can also be used directly when more
//! explicit control is useful.
//!
//! The model is simple:
//!
//! - [`Action`](crate::trampoline::Action) represents one step of a computation
//! - [`done`](crate::trampoline::done) creates a completed step
//! - [`call`](crate::trampoline::call) creates a deferred step that will produce the next
//!   [`Action`](crate::trampoline::Action)
//! - [`run`](crate::trampoline::run) repeatedly evaluates actions until a final value is produced
//!
//! A direct runtime implementation usually consists of:
//!
//! 1. an entry-point function that calls [`run`](crate::trampoline::run)
//! 2. one or more builder functions that return [`Action`](crate::trampoline::Action)
//! 3. recursive transitions expressed by returning another builder's
//!    [`Action`](crate::trampoline::Action)
//!
//! For example:
//!
//! ```rust
//! use tailcall::trampoline;
//!
//! fn is_even(x: u128) -> bool {
//!     trampoline::run(build_is_even_action(x))
//! }
//!
//! fn build_is_even_action(x: u128) -> trampoline::Action<'static, bool> {
//!     trampoline::call(move || {
//!         if x == 0 {
//!             trampoline::done(true)
//!         } else {
//!             build_is_odd_action(x - 1)
//!         }
//!     })
//! }
//!
//! fn build_is_odd_action(x: u128) -> trampoline::Action<'static, bool> {
//!     trampoline::call(move || {
//!         if x == 0 {
//!             trampoline::done(false)
//!         } else {
//!             build_is_even_action(x - 1)
//!         }
//!     })
//! }
//!
//! assert!(is_even(1000));
//! ```
//!
//! The lifetime parameter on [`Action`](crate::trampoline::Action) ties the action to any
//! borrowed data captured by pending steps. For borrowed input, thread that lifetime through the
//! builder functions and finish the computation with [`run`](crate::trampoline::run):
//!
//! ```rust
//! use tailcall::trampoline;
//!
//! fn sum_csv(input: &str) -> u64 {
//!     trampoline::run(build_skip_separators(input.as_bytes(), 0))
//! }
//!
//! fn build_skip_separators<'a>(rest: &'a [u8], total: u64) -> trampoline::Action<'a, u64> {
//!     trampoline::call(move || match rest {
//!         [b' ' | b',', tail @ ..] => build_skip_separators(tail, total),
//!         [] => trampoline::done(total),
//!         _ => build_read_number(rest, total, 0),
//!     })
//! }
//!
//! fn build_read_number<'a>(
//!     rest: &'a [u8],
//!     total: u64,
//!     current: u64,
//! ) -> trampoline::Action<'a, u64> {
//!     trampoline::call(move || match rest {
//!         [digit @ b'0'..=b'9', tail @ ..] => {
//!             let current = current * 10 + u64::from(digit - b'0');
//!             build_read_number(tail, total, current)
//!         }
//!         _ => build_skip_separators(rest, total + current),
//!     })
//! }
//!
//! assert_eq!(sum_csv("10, 20, 3"), 33);
//! ```

use crate::thunk::Thunk;

/// An opaque step in the trampoline runtime.
///
/// Values of this type are usually created with [`call`] and [`done`] and then consumed by
/// [`run`].
/// Users do not inspect or construct the internal representation directly.
pub struct Action<'a, T>(ActionKind<'a, T>);

enum ActionKind<'a, T> {
    Done(T),
    Call(Thunk<'a, Action<'a, T>>),
}

/// Produces a completed [`Action`].
///
/// This is the terminal step in a trampoline computation.
pub const fn done<'a, T>(value: T) -> Action<'a, T> {
    Action(ActionKind::Done(value))
}

/// Produces a pending [`Action`] from a `FnOnce`.
///
/// The closure is executed later by [`run`], and must return the next [`Action`] in the
/// computation.
pub const fn call<'a, T, F>(fn_once: F) -> Action<'a, T>
where
    F: FnOnce() -> Action<'a, T> + 'a,
{
    Action(ActionKind::Call(Thunk::new(fn_once)))
}

/// Runs trampoline actions until they resolve to a final value.
///
/// This is the usual entry point for direct runtime usage.
pub fn run<T>(mut action: Action<'_, T>) -> T {
    loop {
        match action.0 {
            ActionKind::Call(thunk) => action = thunk.call(),
            ActionKind::Done(value) => return value,
        }
    }
}
