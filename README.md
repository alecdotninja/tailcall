# Tailcall

[![Build Status](https://travis-ci.org/alecdotninja/tailcall.svg?branch=master)](https://travis-ci.org/alecdotninja/tailcall)
[![Docs](https://docs.rs/tailcall/badge.svg)](https://docs.rs/tailcall)

Tailcall is a library that adds safe, zero-cost [tail recursion](https://en.wikipedia.org/wiki/Tail_call) to stable Rust.

Eventually, it will be superseded by the [`become` keyword](https://internals.rust-lang.org/t/pre-rfc-explicit-proper-tail-calls/3797/16).

## Installation

Tailcall is distributed as a [crate](https://crates.io/crates/tailcall).

Add this to your `Cargo.toml`:

```toml
[dependencies]
tailcall = "0.1"
```

## Usage

Add the `tailcall` attribute to functions which you would like to use tail recursion:

```rust
use tailcall::tailcall;

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
```

For more detailed information, please see [the docs](https://docs.rs/tailcall).

## Development

Development dependencies, testing, documentation generation, packaging, and distribution are all managed via [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html).

After checking out the repo, run `cargo test` to verify the test suite.
The latest documentation can be generated with `cargo doc`.
Before commiting, please make sure code is formatted canonically with `cargo fmt`.
New versions are released to [crates.io](https://crates.io/crates/tailcall) with `cargo publish`.

## Contributing

Bug reports and pull requests are welcome on [GitHub](https://github.com/alecdotninja/tailcall).

## License

Tailcall is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT), and [COPYRIGHT](COPYRIGHT) for details.
