#![no_std]

use tailcall::{tailcall, Thunk};

#[tailcall]
pub fn countdown(input: u32) -> u32 {
    if input == 0 {
        0
    } else {
        tailcall::call! { countdown(input - 1) }
    }
}

pub fn runtime_countdown(input: u32) -> u32 {
    build_countdown(input).call()
}

fn build_countdown(input: u32) -> Thunk<'static, u32> {
    Thunk::bounce(move || {
        if input == 0 {
            Thunk::value(0)
        } else {
            build_countdown(input - 1)
        }
    })
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::{countdown, runtime_countdown};

    #[cfg(miri)]
    const DEEP_COUNTDOWN: u32 = 10_000;

    #[cfg(not(miri))]
    const DEEP_COUNTDOWN: u32 = 1_000_000;

    #[test]
    fn macro_runtime_handles_deep_recursion_in_no_std_crate() {
        assert_eq!(countdown(DEEP_COUNTDOWN), 0);
    }

    #[test]
    fn manual_runtime_handles_deep_recursion_in_no_std_crate() {
        assert_eq!(runtime_countdown(DEEP_COUNTDOWN), 0);
    }
}
