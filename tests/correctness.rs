use tailcall::*;

fn factorial(input: u64) -> u64 {
    #[tailcall]
    fn factorial_inner(accumulator: u64, input: u64) -> u64 {
        if input > 0 {
            factorial_inner(accumulator * input, input - 1)
        } else {
            accumulator
        }
    }

    factorial_inner(1, input)
}

#[test]
fn test_factorial_correctness() {
    assert_eq!(factorial(0), 1);
    assert_eq!(factorial(1), 1);
    assert_eq!(factorial(2), 2);
    assert_eq!(factorial(3), 6);
    assert_eq!(factorial(4), 24);
}
