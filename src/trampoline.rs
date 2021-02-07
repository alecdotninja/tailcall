//! This module provides a simple, zero-cost [trampoline]. It is designed to be used by the
//! [`tailcall`] macro, but it can also be used manually.
//!
//! # Usage
//!
//! Express the contents of a recusive function as a step function (`Fn(Input) -> Next<Input, Output>`).
//! To guarantee that only a single stack frame will be used at all levels of optimization, annotate it
//! with `#[inline(always)]` attribute. This step function and an initial input can then be passed to
//! [`run`] which will recusively call it until it resolves to an output.
//!
//! ```
//! // fn gcd(a: u64, b: u64) -> u64 {
//! //     if b == 0 {
//! //         a
//! //     } else {
//! //         gcd(b, a % b)
//! //     }
//! // }
//!
//! #[inline(always)]
//! fn gcd_step((a, b): (u64, u64)) -> tailcall::trampoline::Next<(u64, u64), u64> {
//!     if b == 0 {
//!         tailcall::trampoline::Finish(a)
//!     } else {
//!         tailcall::trampoline::Recurse((b, a % b))
//!     }
//! }
//!
//! fn gcd(a: u64, b: u64) -> u64 {
//!
//!     tailcall::trampoline::run(gcd_step, (a, b))
//! }
//! ```
//!
//! [trampoline]: https://en.wikipedia.org/wiki/Tail_call#Through_trampolining
//! [`tailcall`]: ../tailcall_impl/attr.tailcall.html
//! [`run`]: fn.run.html
//!

/// This is the output of the step function. It indicates to [run] what should happen next.
///
/// [run]: fn.run.html
#[derive(Debug)]
pub enum Next<Input, Output> {
    /// This variant indicates that the step function should be run again with the provided input.
    Recurse(Input),

    /// This variant indicates that there are no more steps to be taken and the provided output should be returned.
    Finish(Output),
}

pub use Next::*;

/// Runs a step function aginast a particular input until it resolves to an output.
#[inline(always)]
pub fn run<StepFn, Input, Output>(step: StepFn, mut input: Input) -> Output
where
    StepFn: Fn(Input) -> Next<Input, Output>,
{
    loop {
        match step(input) {
            Recurse(new_input) => {
                input = new_input;
                continue;
            }
            Finish(output) => {
                break output;
            }
        }
    }
}

/// Runs a step function aginast a particular input until it resolves to an output
/// of type `Result<Output, Err>`.
#[inline(always)]
pub fn run_res<StepFn, Input, Output, Err>(step: StepFn, mut input: Input) -> Result<Output, Err>
where
    StepFn: Fn(Input) -> Result<Next<Input, Result<Output, Err>>, Err>,
{
    loop {
        match step(input) {
            Ok(Recurse(new_input)) => {
                input = new_input;
                continue;
            }
            Ok(Finish(output)) => {
                break output;
            }
            Err(err) => {
                break Err(err);
            }
        }
    }
}
