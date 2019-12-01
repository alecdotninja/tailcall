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

For more detailed information (including some limitations), please see [the docs](https://docs.rs/tailcall).

## Implementation

The core idea is to rewrite the function into a loop. Here is the (slightly reformatted) expansion for the `facotrial_inner` example above:

```rust
fn factorial_inner(accumulator: u64, input: u64) -> u64 {
    mod ___tailcall___ {
        pub enum Next<Input, Output> {
            Recurse(Input),
            Finish(Output),
        }

        pub use Next::*;

        #[inline(always)]
        pub fn run<Step, Input, Output>(step: Step, mut input: Input) -> Output
            where Step: Fn(Input) -> Next<Input, Output>
        {
            loop {
                match step(input) {
                    Recurse(new_input) => {
                        input = new_input;
                        continue;
                    },
                    Finish(output) => {
                        break output;
                    }
                }
            }
        }
    }

    ___tailcall___::run(
        #[inline(always)] |(accumulator, input)| {
            ___tailcall___::Finish({
                if input > 0 {
                    return ___tailcall___::Recurse((accumulator * input, input - 1))
                } else {
                    accumulator
                }
            })
        },
        (accumulator, input),
    )
}
```

You can view the exact expansion for the `tailcall` macro in your use-case with `cargo expand`.

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
