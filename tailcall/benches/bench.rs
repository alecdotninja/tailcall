use bencher::*;
use std::boxed::Box;
use tailcall::*;

enum OddnessStep {
    Even(u128),
    Odd(u128),
}

enum BoxThunk<T> {
    Done(T),
    Bounce(Box<dyn FnOnce() -> BoxThunk<T>>),
}

impl<T> BoxThunk<T> {
    fn value(value: T) -> Self {
        Self::Done(value)
    }

    fn bounce<F>(f: F) -> Self
    where
        F: FnOnce() -> Self + 'static,
    {
        Self::Bounce(Box::new(f))
    }

    fn call(mut self) -> T {
        loop {
            match self {
                Self::Done(value) => return value,
                Self::Bounce(next) => self = next(),
            }
        }
    }
}

#[inline(always)]
fn scramble_step(state: u64, n: u64) -> u64 {
    state.rotate_left(7) ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

fn is_even_loop(x: u128) -> bool {
    let mut i: u128 = x;
    let mut even = true;
    while i > 0 {
        i -= 1;
        even = !even;
    }
    even
}

fn is_odd_loop(x: u128) -> bool {
    !is_even_loop(x)
}

fn is_odd_enum_dispatch(x: u128) -> bool {
    let mut step = OddnessStep::Odd(x);

    loop {
        step = match step {
            OddnessStep::Even(0) => return true,
            OddnessStep::Odd(0) => return false,
            OddnessStep::Even(n) => OddnessStep::Odd(n - 1),
            OddnessStep::Odd(n) => OddnessStep::Even(n - 1),
        };
    }
}

fn is_odd_runtime(x: u128) -> bool {
    build_is_odd_runtime(x).call()
}

fn is_odd_box_runtime(x: u128) -> bool {
    build_is_odd_box_runtime(x).call()
}

fn build_is_even_runtime(x: u128) -> runtime::Thunk<'static, bool> {
    runtime::Thunk::bounce(move || {
        if x > 0 {
            build_is_odd_runtime(x - 1)
        } else {
            runtime::Thunk::value(true)
        }
    })
}

fn build_is_odd_runtime(x: u128) -> runtime::Thunk<'static, bool> {
    runtime::Thunk::bounce(move || {
        if x > 0 {
            build_is_even_runtime(x - 1)
        } else {
            runtime::Thunk::value(false)
        }
    })
}

fn build_is_even_box_runtime(x: u128) -> BoxThunk<bool> {
    BoxThunk::bounce(move || {
        if x > 0 {
            build_is_odd_box_runtime(x - 1)
        } else {
            BoxThunk::value(true)
        }
    })
}

fn build_is_odd_box_runtime(x: u128) -> BoxThunk<bool> {
    BoxThunk::bounce(move || {
        if x > 0 {
            build_is_even_box_runtime(x - 1)
        } else {
            BoxThunk::value(false)
        }
    })
}

#[tailcall]
fn is_odd_rec_go(x: u128, odd: bool) -> bool {
    if x > 0 {
        tailcall::call! { is_odd_rec_go(x - 1, !odd) }
    } else {
        odd
    }
}

fn is_odd_rec(x: u128) -> bool {
    is_odd_rec_go(x, false)
}

fn is_odd_rec_thunk(x: u128) -> bool {
    __tailcall_build_is_odd_rec_go_thunk(x, false).call()
}

#[tailcall]
fn is_odd_rec_res_go(x: u64, odd: Result<bool, ()>) -> Result<bool, ()> {
    if x > 0 {
        match odd {
            Ok(odd) => tailcall::call! { is_odd_rec_res_go(x - 1, Ok(!odd)) },
            Err(err) => Err(err),
        }
    } else {
        odd
    }
}

fn is_odd_res_rec(x: u64) -> bool {
    is_odd_rec_res_go(x, Ok(false)).unwrap()
}

// Same as `is_odd_rec_go`, but without the tailcall annotation.
fn is_odd_boom_go(x: u128, odd: bool) -> bool {
    if x > 0 {
        is_odd_boom_go(x - 1, !odd)
    } else {
        odd
    }
}

fn is_odd_boom(x: u128) -> bool {
    is_odd_boom_go(x, false)
}

#[tailcall]
fn is_even_mutrec(x: u128) -> bool {
    if x > 0 {
        tailcall::call! { is_odd_mutrec(x - 1) }
    } else {
        true
    }
}

#[tailcall]
fn is_odd_mutrec(x: u128) -> bool {
    if x > 0 {
        tailcall::call! { is_even_mutrec(x - 1) }
    } else {
        false
    }
}

const ODD_TEST_NUM: u128 = 1000000;
const SCRAMBLE_TEST_NUM: u64 = 1_000_000;

fn bench_oddness_loop(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        black_box(is_odd_loop(black_box(val)));
        val += 1;
    });
}

fn bench_oddness_enum_dispatch(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        black_box(is_odd_enum_dispatch(black_box(val)));
        val += 1;
    });
}

fn bench_oddness_runtime(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        black_box(is_odd_runtime(black_box(val)));
        val += 1;
    });
}

fn bench_oddness_box_runtime(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        black_box(is_odd_box_runtime(black_box(val)));
        val += 1;
    });
}

fn bench_oddness_tailcall_optimized(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        black_box(is_odd_rec(black_box(val)));
        val += 1;
    });
}

fn bench_oddness_tailcall_thunk_builder(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        black_box(is_odd_rec_thunk(black_box(val)));
        val += 1;
    });
}

fn bench_oddness_res_rec(b: &mut Bencher) {
    let mut val: u64 = ODD_TEST_NUM as u64;
    b.iter(|| {
        black_box(is_odd_res_rec(black_box(val)));
        val += 1;
    });
}

fn bench_oddness_boom(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        black_box(is_odd_boom(black_box(val)));
        val += 1;
    });
}

fn bench_oddness_mutrec(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        black_box(is_odd_mutrec(black_box(val)));
        val += 1;
    });
}

fn scramble_loop(n: u64, state: u64) -> u64 {
    let mut n = n;
    let mut state = state;

    while n > 0 {
        state = scramble_step(state, n);
        n -= 1;
    }

    state
}

fn scramble_runtime(n: u64, state: u64) -> u64 {
    build_scramble_runtime(n, state).call()
}

fn build_scramble_runtime(n: u64, state: u64) -> runtime::Thunk<'static, u64> {
    runtime::Thunk::bounce(move || {
        if n > 0 {
            build_scramble_runtime(n - 1, scramble_step(state, n))
        } else {
            runtime::Thunk::value(state)
        }
    })
}

fn scramble_box_runtime(n: u64, state: u64) -> u64 {
    build_scramble_box_runtime(n, state).call()
}

fn build_scramble_box_runtime(n: u64, state: u64) -> BoxThunk<u64> {
    BoxThunk::bounce(move || {
        if n > 0 {
            build_scramble_box_runtime(n - 1, scramble_step(state, n))
        } else {
            BoxThunk::value(state)
        }
    })
}

#[tailcall]
fn scramble_tailcall_go(n: u64, state: u64) -> u64 {
    if n > 0 {
        tailcall::call! { scramble_tailcall_go(n - 1, scramble_step(state, n)) }
    } else {
        state
    }
}

fn scramble_tailcall(n: u64, state: u64) -> u64 {
    scramble_tailcall_go(n, state)
}

fn scramble_tailcall_thunk(n: u64, state: u64) -> u64 {
    __tailcall_build_scramble_tailcall_go_thunk(n, state).call()
}

fn bench_scramble_loop(b: &mut Bencher) {
    let mut val = SCRAMBLE_TEST_NUM;
    b.iter(|| {
        black_box(scramble_loop(
            black_box(val),
            black_box(0xDEAD_BEEF_DEAD_BEEF),
        ));
        val += 1;
    });
}

fn bench_scramble_runtime(b: &mut Bencher) {
    let mut val = SCRAMBLE_TEST_NUM;
    b.iter(|| {
        black_box(scramble_runtime(
            black_box(val),
            black_box(0xDEAD_BEEF_DEAD_BEEF),
        ));
        val += 1;
    });
}

fn bench_scramble_box_runtime(b: &mut Bencher) {
    let mut val = SCRAMBLE_TEST_NUM;
    b.iter(|| {
        black_box(scramble_box_runtime(
            black_box(val),
            black_box(0xDEAD_BEEF_DEAD_BEEF),
        ));
        val += 1;
    });
}

fn bench_scramble_tailcall_optimized(b: &mut Bencher) {
    let mut val = SCRAMBLE_TEST_NUM;
    b.iter(|| {
        black_box(scramble_tailcall(
            black_box(val),
            black_box(0xDEAD_BEEF_DEAD_BEEF),
        ));
        val += 1;
    });
}

fn bench_scramble_tailcall_thunk_builder(b: &mut Bencher) {
    let mut val = SCRAMBLE_TEST_NUM;
    b.iter(|| {
        black_box(scramble_tailcall_thunk(
            black_box(val),
            black_box(0xDEAD_BEEF_DEAD_BEEF),
        ));
        val += 1;
    });
}

benchmark_group!(
    benches,
    bench_oddness_loop,
    bench_oddness_enum_dispatch,
    bench_oddness_runtime,
    bench_oddness_box_runtime,
    bench_oddness_tailcall_optimized,
    bench_oddness_tailcall_thunk_builder,
    bench_oddness_res_rec,
    bench_oddness_boom,
    bench_oddness_mutrec,
    bench_scramble_loop,
    bench_scramble_runtime,
    bench_scramble_box_runtime,
    bench_scramble_tailcall_optimized,
    bench_scramble_tailcall_thunk_builder
);

benchmark_main!(benches);
