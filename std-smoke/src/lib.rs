use tailcall::{runtime::Thunk, tailcall};

#[tailcall]
pub fn countdown(input: u32) -> u32 {
    if input == 0 {
        0
    } else {
        tailcall::call! { countdown(input - 1) }
    }
}

#[tailcall]
pub fn is_even(input: u32) -> bool {
    if input == 0 {
        true
    } else {
        tailcall::call! { is_odd(input - 1) }
    }
}

#[tailcall]
pub fn is_odd(input: u32) -> bool {
    if input == 0 {
        false
    } else {
        tailcall::call! { is_even(input - 1) }
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
mod tests {
    use super::{countdown, is_even, is_odd, runtime_countdown};

    #[cfg(miri)]
    const DEEP_COUNTDOWN: u32 = 10_000;

    #[cfg(not(miri))]
    const DEEP_COUNTDOWN: u32 = 1_000_000;

    #[cfg(miri)]
    const DEEP_PARITY: u32 = 10_001;

    #[cfg(not(miri))]
    const DEEP_PARITY: u32 = 5_000_001;

    #[test]
    fn macro_runtime_handles_deep_recursion_in_std_crate() {
        assert_eq!(countdown(DEEP_COUNTDOWN), 0);
    }

    #[test]
    fn manual_runtime_handles_deep_recursion_in_std_crate() {
        assert_eq!(runtime_countdown(DEEP_COUNTDOWN), 0);
    }

    #[test]
    fn macro_mutual_recursion_handles_deep_inputs_in_std_crate() {
        assert!(is_odd(DEEP_PARITY));
        assert!(!is_even(DEEP_PARITY));
    }
}
