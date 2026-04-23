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
fn skips_leading_separators_with_mutual_recursion() {
    assert_eq!(skip_leading_separators("  ,abc"), 3);
    assert_eq!(skip_leading_separators(", , ,"), 0);
    assert_eq!(skip_leading_separators("abc"), 3);
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

fn skip_leading_separators(input: &str) -> usize {
    build_skip_spaces_action(input.as_bytes()).call()
}

#[doc(hidden)]
#[inline(always)]
fn build_skip_spaces_action(rest: &[u8]) -> Thunk<'_, usize> {
    Thunk::bounce(move || match rest {
        [b' ', tail @ ..] => build_skip_spaces_action(tail),
        [b',', ..] => build_skip_commas_action(rest),
        _ => Thunk::value(rest.len()),
    })
}

#[doc(hidden)]
#[inline(always)]
fn build_skip_commas_action(rest: &[u8]) -> Thunk<'_, usize> {
    Thunk::bounce(move || match rest {
        [b',', tail @ ..] => build_skip_commas_action(tail),
        [b' ', ..] => build_skip_spaces_action(rest),
        _ => Thunk::value(rest.len()),
    })
}
