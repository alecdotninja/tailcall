use std::collections::HashMap;
use std::path::Path;
use tailcall::*;

fn factorial(input: u64) -> u64 {
    #[tailcall]
    fn factorial_inner(accumulator: u64, input: u64) -> u64 {
        if input > 0 {
            tailcall::call! { factorial_inner(accumulator * input, input - 1) }
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

fn memoized_factorial(input: u64, memo: &mut HashMap<u64, u64>) -> u64 {
    #[tailcall]
    fn factorial_inner(accumulator: u64, input: u64, memo: &mut HashMap<u64, u64>) -> u64 {
        memo.insert(input, accumulator);

        if input > 0 {
            tailcall::call! { factorial_inner(accumulator * input, input - 1, memo) }
        } else {
            accumulator
        }
    }

    factorial_inner(1, input, memo)
}

#[tailcall]
#[allow(dead_code)]
fn add_iter<'a, I>(int_iter: I, accum: i32) -> i32
where
    I: Iterator<Item = &'a i32>,
{
    let mut int_iter = int_iter;

    match int_iter.next() {
        Some(i) => tailcall::call! { add_iter(int_iter, accum + i) },
        None => accum,
    }
}

#[test]
fn test_memoized_factorial_correctness() {
    let mut memo = HashMap::new();

    assert_eq!(memoized_factorial(4, &mut memo), 24);
    assert_eq!(memo.get(&0), Some(&24));
    assert_eq!(memo.get(&1), Some(&24));
    assert_eq!(memo.get(&2), Some(&12));
    assert_eq!(memo.get(&3), Some(&4));
    assert_eq!(memo.get(&4), Some(&1));
}

mod qualified_calls {
    use tailcall::tailcall;

    #[tailcall]
    pub fn countdown(input: u64) -> u64 {
        if input > 0 {
            return tailcall::call! { self::countdown(input - 1) };
        }

        input
    }
}

#[test]
fn test_qualified_tailcall_path_and_explicit_return() {
    assert_eq!(qualified_calls::countdown(5), 0);
}

fn gcd_with_trace(a: u64, b: u64) -> (u64, Vec<(u64, u64)>) {
    #[tailcall]
    fn gcd_inner(a: u64, b: u64, trace: &mut Vec<(u64, u64)>) -> u64 {
        trace.push((a, b));

        match b {
            0 => a,
            _ => {
                let next = (b, a % b);
                tailcall::call! { gcd_inner(next.0, next.1, trace) }
            }
        }
    }

    let mut trace = Vec::new();
    let gcd = gcd_inner(a, b, &mut trace);

    (gcd, trace)
}

#[test]
fn test_tailcall_with_nested_match_and_mutable_state() {
    let (gcd, trace) = gcd_with_trace(48, 18);

    assert_eq!(gcd, 6);
    assert_eq!(trace, vec![(48, 18), (18, 12), (12, 6), (6, 0)]);
}

fn sum_csv_numbers(input: &str) -> u64 {
    #[tailcall]
    fn sum_csv_numbers_inner(rest: &[u8], total: u64, current: u64) -> u64 {
        match rest {
            [digit @ b'0'..=b'9', tail @ ..] => {
                let current = current * 10 + u64::from(digit - b'0');
                tailcall::call! { sum_csv_numbers_inner(tail, total, current) }
            }
            [b' ' | b',', tail @ ..] => {
                let total = total + current;
                tailcall::call! { sum_csv_numbers_inner(tail, total, 0) }
            }
            [_other, tail @ ..] => {
                tailcall::call! { sum_csv_numbers_inner(tail, total, current) }
            }
            [] => total + current,
        }
    }

    sum_csv_numbers_inner(input.as_bytes(), 0, 0)
}

#[test]
fn test_tailcall_over_borrowed_input_with_state_machine_logic() {
    assert_eq!(sum_csv_numbers("10, 20,3"), 33);
    assert_eq!(sum_csv_numbers("7  , 8,   9"), 24);
    assert_eq!(sum_csv_numbers("5, x, 11"), 16);
    assert_eq!(sum_csv_numbers(""), 0);
}

#[tailcall]
fn is_even_macro(x: u128) -> bool {
    if x == 0 {
        true
    } else {
        tailcall::call! { is_odd_macro(x - 1) }
    }
}

#[tailcall]
fn is_odd_macro(x: u128) -> bool {
    if x == 0 {
        false
    } else {
        tailcall::call! { is_even_macro(x - 1) }
    }
}

#[test]
fn test_mutual_recursion_via_macros() {
    assert!(is_even_macro(10_000));
    assert!(!is_even_macro(10_001));
    assert!(!is_odd_macro(10_000));
    assert!(is_odd_macro(10_001));
}

struct MethodParity;

impl MethodParity {
    #[tailcall]
    fn is_even(&self, x: u128) -> bool {
        if x == 0 {
            true
        } else {
            tailcall::call! { self.is_odd(x - 1) }
        }
    }

    #[tailcall]
    fn is_odd(&self, x: u128) -> bool {
        if x == 0 {
            false
        } else {
            tailcall::call! { self.is_even(x - 1) }
        }
    }
}

#[test]
fn test_mutual_recursion_via_methods() {
    let parity = MethodParity;

    assert!(parity.is_even(10_000));
    assert!(!parity.is_even(10_001));
    assert!(!parity.is_odd(10_000));
    assert!(parity.is_odd(10_001));
}

#[derive(Default)]
struct MethodAccumulator {
    steps: usize,
}

impl MethodAccumulator {
    #[tailcall]
    fn sum_csv(&mut self, rest: &[u8], total: u64, current: u64) -> u64 {
        self.steps += 1;

        match rest {
            [digit @ b'0'..=b'9', tail @ ..] => {
                let current = current * 10 + u64::from(digit - b'0');
                tailcall::call! { self.sum_csv(tail, total, current) }
            }
            [b' ' | b',', tail @ ..] => {
                let total = total + current;
                tailcall::call! { self.sum_csv(tail, total, 0) }
            }
            [] => total + current,
            [_other, tail @ ..] => {
                tailcall::call! { self.sum_csv(tail, total, current) }
            }
        }
    }
}

#[test]
fn test_mutable_receiver_methods_work_with_tailcall() {
    let mut accumulator = MethodAccumulator::default();
    let total = accumulator.sum_csv(b"10, 20, 3", 0, 0);

    assert_eq!(total, 33);
    assert!(accumulator.steps > 0);
}

#[tailcall]
fn recurse_with_metadata(n: u64) -> u64 {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let len = file.metadata().unwrap().len();

    if n >= 1_000 {
        n
    } else {
        tailcall::call! { recurse_with_metadata(len + n) }
    }
}

#[test]
fn test_issue_18_non_tail_setup_before_recursive_call() {
    let result = recurse_with_metadata(1);

    assert!(result >= 1_000);
}

#[tailcall]
fn mixed_recursion_sum(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => tailcall::call! { mixed_recursion_sum(0) },
        _ if n % 2 == 0 => {
            let partial = mixed_recursion_sum(n - 1);
            n + partial
        }
        _ => tailcall::call! { mixed_recursion_sum(n - 1) },
    }
}

#[test]
fn test_mixed_recursion_allows_plain_non_tail_calls() {
    assert_eq!(mixed_recursion_sum(0), 0);
    assert_eq!(mixed_recursion_sum(1), 0);
    assert_eq!(mixed_recursion_sum(2), 2);
    assert_eq!(mixed_recursion_sum(3), 2);
    assert_eq!(mixed_recursion_sum(4), 6);
    assert_eq!(mixed_recursion_sum(5), 6);
    assert_eq!(mixed_recursion_sum(6), 12);
}
