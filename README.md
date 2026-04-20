# Tailcall

[![CI](https://github.com/alecdotninja/tailcall/actions/workflows/ci.yml/badge.svg)](https://github.com/alecdotninja/tailcall/actions/workflows/ci.yml)
[![Current Crates.io Version](https://img.shields.io/crates/v/tailcall.svg)](https://crates.io/crates/tailcall)
[![Docs](https://docs.rs/tailcall/badge.svg)](https://docs.rs/tailcall)

`tailcall` provides stack-safe tail calls on stable Rust.

It does this with an explicit trampoline runtime backed by a small stack-allocated thunk slot. The
macro API rewrites a function into:

- a public wrapper that calls `tailcall::trampoline::run(...)`
- a hidden builder function that produces `tailcall::trampoline::Action` values

This is still a trampoline approach, but it no longer rewrites recursive calls into a local loop.
Instead, each tail step is represented as a thunk and executed by the runtime.

Eventually, this crate may be superseded by the
[`become` keyword](https://internals.rust-lang.org/t/pre-rfc-explicit-proper-tail-calls/3797/16).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tailcall = "~2"
```

## Usage

Mark a function with `#[tailcall]`, and use `tailcall::call!` at each recursive tail-call site:

```rust
use tailcall::tailcall;

#[tailcall]
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        tailcall::call! { gcd(b, a % b) }
    }
}

assert_eq!(gcd(12, 18), 6);
```

The explicit `tailcall::call!` is part of the API now. Recursive calls are not rewritten
implicitly.

### More Macro Examples

The macro also works well for stateful traversals over borrowed input:

```rust
use tailcall::tailcall;

#[tailcall]
fn sum_csv_numbers(rest: &[u8], total: u64, current: u64) -> u64 {
    match rest {
        [digit @ b'0'..=b'9', tail @ ..] => {
            let current = current * 10 + u64::from(digit - b'0');
            tailcall::call! { sum_csv_numbers(tail, total, current) }
        }
        [b' ' | b',', tail @ ..] => {
            let total = total + current;
            tailcall::call! { sum_csv_numbers(tail, total, 0) }
        }
        [] => total + current,
        [_other, tail @ ..] => {
            tailcall::call! { sum_csv_numbers(tail, total, current) }
        }
    }
}

assert_eq!(sum_csv_numbers(b"10, 20, 3", 0, 0), 33);
```

Most users should only need `#[tailcall]` plus `tailcall::call!`.

### Macro-Based Mutual Recursion

Mutual recursion works through the macro too. Each participating function just needs
`#[tailcall]`, and each tail-call site must use `tailcall::call!`:

```rust
use tailcall::tailcall;

#[tailcall]
fn is_even(x: u128) -> bool {
    if x == 0 {
        true
    } else {
        tailcall::call! { is_odd(x - 1) }
    }
}

#[tailcall]
fn is_odd(x: u128) -> bool {
    if x == 0 {
        false
    } else {
        tailcall::call! { is_even(x - 1) }
    }
}

assert!(is_even(1000));
assert!(is_odd(1001));
```

### Methods

Methods in `impl` blocks work too, including recursive calls written with method syntax on
`self`:

```rust
use tailcall::tailcall;

struct Parity;

impl Parity {
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

let parity = Parity;
assert!(parity.is_even(1000));
```

### Mixed Recursion

Mixed recursion also works in a single `#[tailcall]` function. Only call sites wrapped in
`tailcall::call!` are trampoline-backed; plain recursive calls stay ordinary Rust calls:

```rust
use tailcall::tailcall;

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

assert_eq!(mixed_recursion_sum(6), 12);
```

If only part of a larger algorithm is tail-recursive, it can still be cleaner to put that part in
a helper and annotate the helper:

```rust
use tailcall::tailcall;

fn factorial(n: u64) -> u64 {
    #[tailcall]
    fn factorial_inner(acc: u64, n: u64) -> u64 {
        if n == 0 {
            acc
        } else {
            tailcall::call! { factorial_inner(acc * n, n - 1) }
        }
    }

    factorial_inner(1, n)
}

fn weighted_countdown(n: u64) -> u64 {
    if n <= 3 {
        n + factorial(n)
    } else {
        factorial(n / 2)
    }
}

assert_eq!(weighted_countdown(3), 9);
assert_eq!(weighted_countdown(8), 24);
```

This helper pattern is often the cleanest approach when one inner phase is tail-recursive and the
rest of the algorithm is not.

### Advanced: Direct Runtime

The runtime can also be used directly:

```rust
use tailcall::trampoline;

fn is_even(x: u128) -> bool {
    trampoline::run(build_is_even_action(x))
}

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
```

The direct runtime is still useful as an escape hatch for advanced manual control. For example,
one function can skip separators while another reads digits from the same slice, passing control
back and forth through `trampoline::Action`.

## Macro Expansion Shape

At a high level, this:

```rust
#[tailcall]
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        tailcall::call! { gcd(b, a % b) }
    }
}
```

becomes roughly:

```rust
fn gcd(a: u64, b: u64) -> u64 {
    tailcall::trampoline::run(__tailcall_build_gcd(a, b))
}

#[inline(always)]
fn __tailcall_build_gcd<'tailcall>(a: u64, b: u64) -> tailcall::trampoline::Action<'tailcall, u64> {
    tailcall::trampoline::call(move || {
        if b == 0 {
            tailcall::trampoline::done(a)
        } else {
            __tailcall_build_gcd(b, a % b)
        }
    })
}
```

The exact expansion is different in edge cases, but this is the core model.

## Limitations

Current macro limitations:

- Tail-call sites must use `tailcall::call! { path(args...) }` or `tailcall::call! { self.method(args...) }`.
- Function arguments must use simple identifier patterns.
- `?` is not supported inside `#[tailcall]` functions on stable Rust. Use `match` or explicit
  early returns instead.
- Trait methods are not supported yet.
- In mixed recursion, only `tailcall::call!` sites are trampoline-backed; plain recursive calls
  still use the native call stack.
- `async fn` and `const fn` are not supported.

The runtime itself can be used directly if the macro is too restrictive for a particular use case.

## Safety Notes

The thunk runtime uses unsafe code internally to type-erase `FnOnce` values into a fixed-size stack
slot. The current test suite includes:

- ordinary correctness tests
- stack-behavior tests
- destructor-behavior tests
- Miri runs over the runtime-oriented tests

## Development

Common commands:

```bash
cargo test
cargo +nightly miri test --all
cargo fmt --all
cargo clippy --all
```

The stack-depth test is skipped under Miri because it measures backtrace shape rather than memory
safety.

## Release Process

This workspace publishes two crates:

- `tailcall-impl`
- `tailcall`

Release them together.

1. Update the workspace version in the root [Cargo.toml](Cargo.toml).
2. Update the matching versions in the root `workspace.dependencies` section.
3. Update the installation snippet in this README if the major version changed.
4. Run:

```bash
cargo test
cargo +nightly miri test --all
cargo bench --all --no-run
```

5. Publish `tailcall-impl` first:

```bash
cargo publish -p tailcall-impl
```

6. Publish `tailcall` after `tailcall-impl` is available on crates.io:

```bash
cargo publish -p tailcall
```

7. Tag the release from `main`:

```bash
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

## Contributing

Bug reports and pull requests are welcome on [GitHub](https://github.com/alecdotninja/tailcall).

## License

Tailcall is distributed under the terms of both the MIT license and the Apache License (Version
2.0).

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and [COPYRIGHT](COPYRIGHT) for
details.
