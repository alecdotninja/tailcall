use tailcall::*;

/// Factorial artificial wrapped in a Option
fn factorial(input: u64) -> Option<u64> {
    #[tailcall]
    fn factorial_inner(accumulator: u64, input: u64) -> Option<u64> {
        if input > 0 {
            tailcall::call! { factorial_inner(accumulator * input, input - 1) }
        } else {
            Some(accumulator)
        }
    }

    factorial_inner(1, input)
}

#[tailcall]
#[allow(dead_code)]
fn add_iter<'a, I>(int_iter: I, accum: i32) -> Option<i32>
where
    I: Iterator<Item = &'a i32>,
{
    let mut int_iter = int_iter;

    match int_iter.next() {
        Some(i) => tailcall::call! { add_iter(int_iter, accum + i) },
        None => Some(accum),
    }
}

#[test]
fn factorial_option_runs() {
    assert_eq!(factorial(0).unwrap(), 1);
    assert_eq!(factorial(1).unwrap(), 1);
    assert_eq!(factorial(2).unwrap(), 2);
    assert_eq!(factorial(3).unwrap(), 6);
    assert_eq!(factorial(4).unwrap(), 24);
}
