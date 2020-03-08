# Tailcall

[![Build Status](https://travis-ci.org/alecdotninja/tailcall.svg?branch=master)](https://travis-ci.org/alecdotninja/tailcall)
[![Current Crates.io Version](https://img.shields.io/crates/v/tailcall.svg)](https://crates.io/crates/tailcall)
[![Docs](https://docs.rs/tailcall/badge.svg)](https://docs.rs/tailcall)

Tailcall is a library that adds safe, zero-cost [tail recursion](https://en.wikipedia.org/wiki/Tail_call) to stable Rust.

Eventually, it will be superseded by the [`become` keyword](https://internals.rust-lang.org/t/pre-rfc-explicit-proper-tail-calls/3797/16).

## Installation

Tailcall is distributed as a [crate](https://crates.io/crates/tailcall).

Add this to your `Cargo.toml`:

```toml
[dependencies]
tailcall = "0.2"
```

## Usage

Add the `tailcall` attribute to functions which you would like to use tail recursion:

```rust
use tailcall::tailcall;

#[tailcall]
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}
```

For more detailed information (including some limitations), please see [the docs](https://docs.rs/crates/tailcall).

## Implementation

The core idea is to rewrite the function into a loop using the [trampoline approach](https://en.wikipedia.org/wiki/Tail_call#Through_trampolining).
Here is the (slightly reformatted) expansion for the `gcd` example above:

```rust
fn gcd(a: u64, b: u64) -> u64 {
    tailcall::trampoline::run(
        #[inline(always)] |(a, b)| {
            tailcall::trampoline::Finish({
                if b == 0 {
                    a
                } else {
                    return tailcall::trampoline::Recurse((b, a % b))
                }
            })
        },
        (a, b),
    )
}
```

You can view the exact expansion for the `tailcall` macro in your use-case with `cargo expand`.

## Development

Development dependencies, testing, documentation generation, packaging, and distribution are all managed via [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html).

After checking out the repo, run `cargo test` to verify the test suite.
The latest documentation can be generated with `cargo doc`.
Before commiting, please make sure code is formatted canonically with `cargo fmt` and passes all lints with `cargo clippy`.
New versions are released to [crates.io](https://crates.io/crates/tailcall) with `cargo publish`.

## Contributing

Bug reports and pull requests are welcome on [GitHub](https://github.com/alecdotninja/tailcall).

## License

Tailcall is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT), and [COPYRIGHT](COPYRIGHT) for details.
