use tailcall::{slot, trampoline};

#[test]
fn factorial_in_new_runtime() {
    assert!(factorial(5) == 120);
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
