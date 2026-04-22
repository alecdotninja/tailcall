# Tailcall

[![CI](https://github.com/alecdotninja/tailcall/actions/workflows/ci.yml/badge.svg)](https://github.com/alecdotninja/tailcall/actions/workflows/ci.yml)
[![Current Crates.io Version](https://img.shields.io/crates/v/tailcall.svg)](https://crates.io/crates/tailcall)
[![Docs](https://docs.rs/tailcall/badge.svg)](https://docs.rs/tailcall)

`tailcall` lets you write deeply recursive functions without blowing the stack—on stable Rust.

It provides **explicit, stack-safe tail calls** using a lightweight trampoline runtime, with a macro that keeps usage ergonomic.

The runtime crate is `no_std`, so it can be used on targets without the standard library.

If the proposed [`become` keyword](https://internals.rust-lang.org/t/pre-rfc-explicit-proper-tail-calls/3797/16) is ever stabilized, it will likely be the preferred solution for proper tail calls.

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

This runs in constant stack space, even for very large inputs.

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

## Cost

A common alternative for stack-safe recursion in Rust is to box each step. That can offer a similar
interface, but it introduces allocation and indirection on every recursive step.

`tailcall` keeps each step inline instead, so the main cost is:

* an extra indirect call per `Thunk::bounce` step

In some cases, that cost disappears entirely. If a simple free function or inherent method only
tail-calls itself directly, `#[tailcall]` can lower it to an inline `loop`.

---

### Rough Performance Shape

On a simple benchmark (relative to a handwritten loop):

* handwritten loop: **1.0×**
* `#[tailcall]` (inline loop): **~1.0×**
* `#[tailcall]` (Thunk runtime): **~3.2× slower**
* boxed runtime: **~14× slower**

This is just a local measurement, but the general pattern holds:

* direct self-recursion can optimize down to loop-like performance
* the `Thunk` runtime is slower, but supports more complex cases
* heap-allocating approaches are slower again

---

### Tradeoff

The slower path is also the more flexible one.

It’s what allows `tailcall` to support:

* mutual recursion
* borrowed-state builders
* recursive control flow that doesn’t collapse into a single loop

If your recursion is simple, you get loop-like performance.

If it’s not, you still get stack safety—without paying for heap allocation.

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

For simple direct self-recursion, the macro can compile free functions and inherent methods to an
inline loop. Mutual recursion and other more complex cases continue to use the hidden `Thunk`
builder automatically.

For methods, that optimized path works by aliasing the receiver once, rebinding the
non-receiver arguments as loop state, and turning each direct self tail call into "assign the
next arguments, then continue the loop".

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

If the function only tail-calls itself directly, this pattern is also the one that enables the
inline-loop optimization.

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
    fn is_even(&self, x: u32) -> bool {
        if x == 0 {
            true
        } else {
            tailcall::call! { self.is_odd(x - 1) }
        }
    }

    #[tailcall]
    fn is_odd(&self, x: u32) -> bool {
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

The macro is just a thin layer over `runtime::Thunk`.

A `runtime::Thunk<T>` is a fixed-size deferred value from a computation, so it can live on the
stack. It may contain the value directly or a type-erased closure that will eventually produce the
value.

On 64-bit targets, the current runtime keeps `Thunk` at 32 bytes. It does that by storing deferred
closures in a small inline slot, which means manual `Thunk` values and macro-generated helpers can
only capture a limited amount of data before construction panics.

Pending `Thunk` values still preserve normal destructor-on-drop behavior for captured values.

You build a chain of steps, then execute it with `.call()`.

### Core constructors

* `Thunk::value(x)` — final result
* `Thunk::new(f)` — deferred computation returning a value
* `Thunk::bounce(f)` — deferred computation returning another `Thunk` (**this is what enables stack safety**)

---

### Example

```rust
use tailcall::runtime::Thunk;

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

### Explicit Tail Calls

Tail-recursive transitions must be written with `tailcall::call!`.

Plain recursive calls are left alone, which means they still use the native Rust call stack.

### Simple Argument Patterns

`#[tailcall]` currently only supports simple identifier arguments.

Patterns in function parameters are not rewritten by the macro.

### `?` Is Not Supported

The `?` operator is not supported inside `#[tailcall]` functions on stable Rust.

Use `match` or explicit early returns instead.

### Trait Methods

Methods in ordinary `impl` blocks are supported.

Trait methods are not supported.

### `async fn` and `const fn`

`#[tailcall]` does not support `async fn` or `const fn`.

### Closure Size Limit

Each deferred closure is stored in a fixed-size inline slot (~16 bytes).

**Closures that exceed that size panic when the `Thunk` is constructed.**

Macro-generated helper thunks are subject to the same limit, so functions with enough arguments or captured state can also exceed it.

---

## Development

For normal development, the main local checks are:

```bash
cargo test
cargo test --doc
cargo +nightly miri test --all
cargo fmt --all -- --check
cargo clippy --all
cargo doc --no-deps
```

If you are changing the public docs or runtime internals, it is also worth doing a quick end-to-end smoke test from a fresh crate that depends on the published version from crates.io.

---

## Publishing

`tailcall` and `tailcall-impl` are released together.

1. Update the shared workspace version in `Cargo.toml`.
   Also update the matching `tailcall` and `tailcall-impl` entries in `[workspace.dependencies]`.
2. Run the release checks:

```bash
cargo test
cargo test --doc
cargo doc --no-deps
```

3. Commit the release version bump.
4. Publish `tailcall-impl` first:

```bash
cargo publish -p tailcall-impl
```

5. Publish `tailcall` after the proc-macro crate is available:

```bash
cargo publish -p tailcall
```

6. Tag the release from `main` and push the commit and tag:

```bash
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

---

## License

Tailcall is distributed under the terms of both the MIT license and the Apache License (Version
2.0).

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and [COPYRIGHT](COPYRIGHT) for
details.
