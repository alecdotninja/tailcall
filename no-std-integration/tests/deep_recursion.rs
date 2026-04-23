use no_std_integration::{countdown, runtime_countdown};

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
