use tailcall::trampoline;

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

fn is_even(x: u128) -> bool {
    trampoline::run(build_is_even_action(x))
}

#[doc(hidden)]
#[inline(always)]
fn build_is_even_action(x: u128) -> trampoline::Action<'static, bool> {
    trampoline::call(move || {
        if x > 0 {
            build_is_odd_action(x - 1)
        } else {
            trampoline::done(true)
        }
    })
}

fn is_odd(x: u128) -> bool {
    trampoline::run(build_is_odd_action(x))
}

#[doc(hidden)]
#[inline(always)]
fn build_is_odd_action(x: u128) -> trampoline::Action<'static, bool> {
    trampoline::call(move || {
        if x > 0 {
            build_is_even_action(x - 1)
        } else {
            trampoline::done(false)
        }
    })
}
