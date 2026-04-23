use std_integration::{countdown, is_even, is_odd, runtime_countdown};

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
