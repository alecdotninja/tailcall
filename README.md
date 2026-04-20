# Tailcall

[![CI](https://github.com/alecdotninja/tailcall/actions/workflows/ci.yml/badge.svg)](https://github.com/alecdotninja/tailcall/actions/workflows/ci.yml)
[![Current Crates.io Version](https://img.shields.io/crates/v/tailcall.svg)](https://crates.io/crates/tailcall)
[![Docs](https://docs.rs/tailcall/badge.svg)](https://docs.rs/tailcall)

`tailcall` lets you write deeply recursive functions in Rust without blowing the stack—on stable Rust.

It provides **explicit, stack-safe tail calls** using a lightweight trampoline runtime, with a macro that keeps usage ergonomic.

---

## Installation

```toml
[dependencies]
tailcall = "~2"
```

---

## Quick Example

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

That’s the core API:

* mark the function with `#[tailcall]`
* wrap recursive tail calls with `tailcall::call!`

---

## When to Use This

`tailcall` is useful when:

* you want to write **naturally recursive code** without risking stack overflow
* converting to loops would make the code harder to read
* you’re working with **mutual recursion** or recursive traversals
* you want stack safety without nightly features

It may not be ideal when:

* a simple loop is clearer
* you need maximum performance (there is some trampoline overhead)

---

## How It Works (Briefly)

Rust does not guarantee tail call optimization. Deep recursion can overflow the stack.

`tailcall` avoids this by using a **trampoline**:

* each recursive step returns a deferred computation (`Thunk`)
* the runtime repeatedly executes those steps in a loop
* no additional stack frames are created

The key operation is:

* `Thunk::bounce(...)` — produces the *next step* instead of recursing

This turns recursion into iteration under the hood.

---

## Macro Usage

Most users only need the macro.

### Basic Pattern

```rust
#[tailcall]
fn f(...) -> T {
    if done {
        result
    } else {
        tailcall::call! { f(next_args...) }
    }
}
```

Only calls wrapped in `tailcall::call!` are stack-safe.

---

### Mutual Recursion

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
```

---

### Methods

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
```

---

### Mixed Recursion

Only `tailcall::call!` sites are trampoline-backed:

```rust
use tailcall::tailcall;

#[tailcall]
fn sum(n: u64) -> u64 {
    match n {
        0 => 0,
        _ if n % 2 == 0 => {
            let partial = sum(n - 1); // normal recursion
            n + partial
        }
        _ => tailcall::call! { sum(n - 1) },
    }
}
```

---

### Recommended Pattern: Tail-Recursive Helper

```rust
use tailcall::tailcall;

fn factorial(n: u64) -> u64 {
    #[tailcall]
    fn inner(acc: u64, n: u64) -> u64 {
        if n == 0 {
            acc
        } else {
            tailcall::call! { inner(acc * n, n - 1) }
        }
    }

    inner(1, n)
}
```

---

## Using the Runtime Directly

The macro is just a thin layer over `Thunk`.

A `Thunk<T>` is a deferred value from a computation.

You build a chain of steps, then execute it with `.call()`.

### Core constructors

* `Thunk::value(x)` — final result
* `Thunk::new(f)` — deferred computation returning a value
* `Thunk::bounce(f)` — deferred computation returning another `Thunk` (**this is what enables stack safety**)

---

### Example

```rust
use tailcall::Thunk;

fn count_down(n: u64) -> u64 {
    build(n).call()
}

fn build(n: u64) -> Thunk<'static, u64> {
    Thunk::bounce(move || {
        if n == 0 {
            Thunk::value(0)
        } else {
            build(n - 1)
        }
    })
}
```

`Thunk::bounce` ensures each step returns control to the runtime loop instead of growing the call stack.

---

## What the Macro Generates

At a high level, this:

```rust
#[tailcall]
fn f(...) -> T { ... }
```

becomes:

* a wrapper that calls `.call()`
* a hidden builder that returns `Thunk<T>`

So the macro:

* rewrites your function into a trampoline-compatible form
* leaves control flow and logic unchanged

---

## Limitations

* Tail calls must use `tailcall::call!`
* Only simple argument patterns are supported
* `?` is not supported inside `#[tailcall]` functions (use `match`)
* Trait methods are not supported
* `async fn` and `const fn` are not supported
* Only `tailcall::call!` sites are stack-safe in mixed recursion

### Closure Size Limit

Each deferred closure is stored in a fixed-size inline slot (~48 bytes).

If a closure captures more than that, constructing the `Thunk` will panic at runtime.
Macro-generated helper thunks are subject to the same limit, so functions with enough arguments or
captured state can also exceed it.

---

## Notes

This approach mirrors proposed language-level tail calls (e.g. `become`), and provides a practical solution on stable Rust today.

---

## Development

```bash
cargo test
cargo +nightly miri test --all
cargo fmt --all
cargo clippy --all
```

---

## License

Tailcall is distributed under the terms of both the MIT license and the Apache License (Version
2.0).

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and [COPYRIGHT](COPYRIGHT) for
details.
