use std::io::Error;
use tailcall::*;

/// Factorial artificial wrapped in a Result
fn factorial_ex(input: u64) -> Result<u64, Error> {
    fn factorial_inner(
        accumulator: Result<u64, Error>,
        input: Result<u64, Error>,
    ) -> Result<u64, Error> {
        tailcall::trampoline::run_res(
            #[inline(always)]
            |(accumulator, input)| {
                Ok(tailcall::trampoline::Finish({
                    let inp = input?;
                    let acc = accumulator?;
                    if inp > 0 {
                        return Ok(tailcall::trampoline::Recurse((Ok(acc * inp), Ok(inp - 1))));
                    } else {
                        Ok(acc)
                    }
                }))
            },
            (accumulator, input),
        )
    }
    factorial_inner(Ok(1), Ok(input))
}


#[test]
fn factorial_ex_result_runs() {
    assert_eq!(factorial_ex(0).unwrap(), 1);
    assert_eq!(factorial_ex(1).unwrap(), 1);
    assert_eq!(factorial_ex(2).unwrap(), 2);
    assert_eq!(factorial_ex(3).unwrap(), 6);
    assert_eq!(factorial_ex(4).unwrap(), 24);
}
