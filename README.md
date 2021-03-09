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
tailcall = "0.1.5"
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

For more detailed information (including some limitations), please see [the docs](https://docs.rs/tailcall).

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

## Benchmarks

There are a few benchmarks available; currently the benchmarks demonstrate that for
single-function tail-recursion, performance is the same as using a loop. Mutual
recursion runs, but suffers penalties.

```
$ cargo bench
    Finished bench [optimized] target(s) in 0.05s
     Running target/release/deps/tailcall-b55b2bddb07cb046

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running target/release/deps/bench-b8ab29e7ebef8d8d

running 4 tests
test bench_oddness_boom   ... bench:           6 ns/iter (+/- 0)
test bench_oddness_loop   ... bench:           6 ns/iter (+/- 0)
test bench_oddness_mutrec ... bench:   4,509,915 ns/iter (+/- 7,095,455)
test bench_oddness_rec    ... bench:           3 ns/iter (+/- 0)

test result: ok. 0 passed; 0 failed; 0 ignored; 4 measured; 0 filtered out
```

If the optimization level is set to zero so that `bench_oddness_boom` isn't cleverly
optimized away, it blows the stack as expected:

```
$ RUSTFLAGS="-C opt-level=0" cargo bench _boom
    Finished bench [optimized] target(s) in 0.05s
     Running target/release/deps/tailcall-b55b2bddb07cb046

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running target/release/deps/bench-b8ab29e7ebef8d8d

running 1 test

thread 'main' has overflowed its stack
fatal runtime error: stack overflow
```

In fact the same occurs when running `RUSTFLAGS="-C opt-level=0" cargo bench _mutrec`
, indicating mutual recursion can also blow the stack, but the `loop` and tailrec-enabled
single-function, tail-recursive functions enjoy TCO:

```
$ RUSTFLAGS="-C opt-level=0" cargo bench _loop
    Finished bench [optimized] target(s) in 0.06s
     Running target/release/deps/tailcall-b55b2bddb07cb046

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running target/release/deps/bench-b8ab29e7ebef8d8d

running 1 test
test bench_oddness_loop   ... bench:   4,514,730 ns/iter (+/- 7,498,984)

test result: ok. 0 passed; 0 failed; 0 ignored; 1 measured; 3 filtered out


$ RUSTFLAGS="-C opt-level=0" cargo bench _rec
    Finished bench [optimized] target(s) in 0.05s
     Running target/release/deps/tailcall-b55b2bddb07cb046

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running target/release/deps/bench-b8ab29e7ebef8d8d

running 1 test
test bench_oddness_rec    ... bench:  22,416,962 ns/iter (+/- 16,083,896)

test result: ok. 0 passed; 0 failed; 0 ignored; 1 measured; 3 filtered out
```


## Contributing

Bug reports and pull requests are welcome on [GitHub](https://github.com/alecdotninja/tailcall).

## License

Tailcall is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT), and [COPYRIGHT](COPYRIGHT) for details.
