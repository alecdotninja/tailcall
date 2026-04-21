use tailcall::Thunk;

#[test]
fn is_even_redux() {
    assert!(is_even(1000));
    assert!(!is_even(1001));
}

#[test]
fn is_odd_redux() {
    assert!(!is_odd(1000));
    assert!(is_odd(1001));
}

#[test]
fn sums_csv_numbers_with_mutual_recursion() {
    assert_eq!(sum_csv_numbers("10, 20,3"), 33);
    assert_eq!(sum_csv_numbers("7  , 8,   9"), 24);
    assert_eq!(sum_csv_numbers(""), 0);
}

fn is_even(x: u128) -> bool {
    build_is_even_action(x).call()
}

#[doc(hidden)]
#[inline(always)]
fn build_is_even_action(x: u128) -> Thunk<'static, bool> {
    Thunk::bounce(move || {
        if x > 0 {
            build_is_odd_action(x - 1)
        } else {
            Thunk::value(true)
        }
    })
}

fn is_odd(x: u128) -> bool {
    build_is_odd_action(x).call()
}

#[doc(hidden)]
#[inline(always)]
fn build_is_odd_action(x: u128) -> Thunk<'static, bool> {
    Thunk::bounce(move || {
        if x > 0 {
            build_is_even_action(x - 1)
        } else {
            Thunk::value(false)
        }
    })
}

fn sum_csv_numbers(input: &str) -> u64 {
    build_skip_separators_action(input.as_bytes(), 0).call()
}

#[doc(hidden)]
#[inline(always)]
fn build_skip_separators_action(rest: &[u8], total: u64) -> Thunk<'_, u64> {
    Thunk::bounce(move || match rest {
        [b' ' | b',', tail @ ..] => build_skip_separators_action(tail, total),
        [] => Thunk::value(total),
        _ => build_read_number_action(rest, total, 0),
    })
}

#[doc(hidden)]
#[inline(always)]
fn build_read_number_action(rest: &[u8], total: u64, current: u64) -> Thunk<'_, u64> {
    Thunk::bounce(move || match rest {
        [digit @ b'0'..=b'9', tail @ ..] => {
            let current = current * 10 + u64::from(digit - b'0');
            build_read_number_action(tail, total, current)
        }
        _ => build_skip_separators_action(rest, total + current),
    })
}
