use tailcall::*;

/// Factorial artificial wrapped in a Option
fn factorial(input: u64) -> Option<u64> {
    #[tailcall]
    fn factorial_inner(accumulator: Option<u64>, input: Option<u64>) -> Option<u64> {
        let inp = input?;
        let acc = accumulator?;
        if inp > 0 {
            factorial_inner(Some(acc * inp), Some(inp - 1))
        } else {
            Some(acc)
        }
    }

    factorial_inner(Some(1), Some(input))
}

#[test]
fn factorial_option_runs() {
    assert_eq!(factorial(0).unwrap(), 1);
    assert_eq!(factorial(1).unwrap(), 1);
    assert_eq!(factorial(2).unwrap(), 2);
    assert_eq!(factorial(3).unwrap(), 6);
    assert_eq!(factorial(4).unwrap(), 24);
}
