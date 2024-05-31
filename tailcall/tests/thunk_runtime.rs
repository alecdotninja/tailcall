use tailcall::{slot, trampoline};

#[test]
fn factorial_in_new_runtime() {
    assert!(factorial(5) == 120);
}

#[test]
fn is_even_in_new_runtime() {
    assert!(is_even(0));
    assert!(!is_even(1));
    assert!(is_even(2));
    assert!(!is_even(3));
    assert!(is_even(4));
    assert!(!is_even(5));
}

#[test]
fn is_odd_in_new_runtime() {
    assert!(!is_odd(0));
    assert!(is_odd(1));
    assert!(!is_odd(2));
    assert!(is_odd(3));
    assert!(!is_odd(4));
    assert!(is_odd(5));
}

fn factorial(input: u64) -> u64 {
    #[inline(always)]
    fn call_factorial_inner<'slot>(
        slot: &'slot mut slot::Slot,
        accumulator: u64,
        input: u64,
    ) -> trampoline::Action<'slot, u64> {
        trampoline::call(slot, move |slot| {
            if input == 0 {
                return trampoline::done(slot, accumulator);
            }

            return call_factorial_inner(slot, accumulator * input, input - 1);
        })
    }

    fn factorial_inner(accumulator: u64, input: u64) -> u64 {
        trampoline::run(move |slot| call_factorial_inner(slot, accumulator, input))
    }

    factorial_inner(1, input)
}

fn is_even(x: u128) -> bool {
    trampoline::run(move |slot| build_is_even_action(slot, x))
}

#[doc(hidden)]
#[inline(always)]
fn build_is_even_action<'slot>(
    slot: &'slot mut slot::Slot,
    x: u128,
) -> trampoline::Action<'slot, bool> {
    trampoline::call(slot, move |slot| {
        if x > 0 {
            build_is_odd_action(slot, x - 1)
        } else {
            trampoline::done(slot, true)
        }
    })
}

fn is_odd(x: u128) -> bool {
    trampoline::run(move |slot| build_is_odd_action(slot, x))
}

#[doc(hidden)]
#[inline(always)]
fn build_is_odd_action<'slot>(
    slot: &'slot mut slot::Slot,
    x: u128,
) -> trampoline::Action<'slot, bool> {
    trampoline::call(slot, move |slot| {
        if x > 0 {
            build_is_even_action(slot, x - 1)
        } else {
            trampoline::done(slot, false)
        }
    })
}
