use tailcall::tailcall;

#[tailcall]
fn fibonacci_go(remaining: u32, current: u128, next: u128) -> u128 {
    if remaining == 0 {
        current
    } else {
        tailcall::call! { fibonacci_go(remaining - 1, next, current + next) }
    }
}

fn fibonacci(n: u32) -> u128 {
    fibonacci_go(n, 0, 1)
}

#[test]
fn fibonacci_uses_accumulator_pair() {
    assert_eq!(fibonacci(0), 0);
    assert_eq!(fibonacci(1), 1);
    assert_eq!(fibonacci(10), 55);
    assert_eq!(fibonacci(40), 102_334_155);
}

#[tailcall]
fn balanced_parentheses_go(rest: &[u8], depth: usize) -> bool {
    match rest {
        [] => depth == 0,
        [b'(', tail @ ..] => {
            tailcall::call! { balanced_parentheses_go(tail, depth + 1) }
        }
        [b')', tail @ ..] if depth > 0 => {
            tailcall::call! { balanced_parentheses_go(tail, depth - 1) }
        }
        [b')', ..] => false,
        [_other, tail @ ..] => {
            tailcall::call! { balanced_parentheses_go(tail, depth) }
        }
    }
}

fn has_balanced_parentheses(input: &str) -> bool {
    balanced_parentheses_go(input.as_bytes(), 0)
}

#[test]
fn balanced_parentheses_scans_borrowed_input() {
    assert!(has_balanced_parentheses("(a + b) * (c + d)"));
    assert!(has_balanced_parentheses("no parentheses"));
    assert!(!has_balanced_parentheses("(()"));
    assert!(!has_balanced_parentheses("())("));
}

enum Tree {
    Leaf(i64),
    Branch(Box<Tree>, Box<Tree>),
}

fn sum_tree(tree: &Tree) -> i64 {
    let mut pending = vec![tree];
    sum_tree_go(&mut pending, 0)
}

#[tailcall]
fn sum_tree_go<'a>(pending: &mut Vec<&'a Tree>, total: i64) -> i64 {
    match pending.pop() {
        Some(Tree::Leaf(value)) => {
            tailcall::call! { sum_tree_go(pending, total + value) }
        }
        Some(Tree::Branch(left, right)) => {
            pending.push(right);
            pending.push(left);
            tailcall::call! { sum_tree_go(pending, total) }
        }
        None => total,
    }
}

#[test]
fn tree_sum_uses_explicit_worklist() {
    let tree = Tree::Branch(
        Box::new(Tree::Branch(
            Box::new(Tree::Leaf(2)),
            Box::new(Tree::Leaf(3)),
        )),
        Box::new(Tree::Branch(
            Box::new(Tree::Leaf(5)),
            Box::new(Tree::Branch(
                Box::new(Tree::Leaf(7)),
                Box::new(Tree::Leaf(11)),
            )),
        )),
    );

    assert_eq!(sum_tree(&tree), 28);
}

fn digits(number: u64) -> Vec<u8> {
    let mut output = Vec::new();
    digits_go(number, &mut output);
    output
}

#[tailcall]
fn digits_go(number: u64, output: &mut Vec<u8>) {
    output.push((number % 10) as u8);
    let number = number / 10;

    if number == 0 {
        output.reverse()
    } else {
        tailcall::call! { digits_go(number, output) }
    }
}

#[test]
fn digits_builds_output_through_mutable_accumulator() {
    assert_eq!(digits(0), vec![0]);
    assert_eq!(digits(9), vec![9]);
    assert_eq!(digits(12_345), vec![1, 2, 3, 4, 5]);
}
