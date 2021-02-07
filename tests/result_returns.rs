use std::io::Error;
use tailcall::*;

/// Factorial artificial wrapped in a Result
fn factorial(input: u64) -> Result<u64, Error> {
    #[tailcall_res]
    fn factorial_inner(
        accumulator: Result<u64, Error>,
        input: Result<u64, Error>,
    ) -> Result<u64, Error> {
        let inp = input?;
        let acc = accumulator?;
        if inp > 0 {
            factorial_inner(Ok(acc * inp), Ok(inp - 1))
        } else {
            Ok(acc)
        }
    }

    factorial_inner(Ok(1), Ok(input))
}

#[test]
fn factorial_result_runs() {
    assert_eq!(factorial(0).unwrap(), 1);
    assert_eq!(factorial(1).unwrap(), 1);
    assert_eq!(factorial(2).unwrap(), 2);
    assert_eq!(factorial(3).unwrap(), 6);
    assert_eq!(factorial(4).unwrap(), 24);
}
