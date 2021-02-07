use bencher::*;
use tailcall::*;

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

#[tailcall]
fn is_odd_rec_go(x: u128, odd: bool) -> bool {
    if x > 0 {
        is_odd_rec_go(x - 1, !odd)
    } else {
        odd
    }
}

fn is_odd_rec(x: u128) -> bool {
    is_odd_rec_go(x, false)
}

#[tailcall_res]
fn is_odd_rec_res_go(x: u128, odd: Result<bool, ()>) -> Result<bool, ()> {
    if x > 0 {
        is_odd_rec_res_go(x - 1, Ok(!odd?))
    } else {
        odd
    }
}

fn is_odd_res_rec(x: u128) -> bool {
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
        is_odd_mutrec(x - 1)
    } else {
        true
    }
}

#[tailcall]
fn is_odd_mutrec(x: u128) -> bool {
    if x > 0 {
        is_even_mutrec(x - 1)
    } else {
        false
    }
}

const ODD_TEST_NUM: u128 = 1000000;

fn bench_oddness_loop(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        is_odd_loop(val);
        val += 1;
    });
}

fn bench_oddness_rec(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        is_odd_rec(val);
        val += 1;
    });
}

fn bench_oddness_res_rec(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        is_odd_res_rec(val);
        val += 1;
    });
}

fn bench_oddness_boom(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        is_odd_boom(val);
        val += 1;
    });
}

fn bench_oddness_mutrec(b: &mut Bencher) {
    let mut val: u128 = ODD_TEST_NUM;
    b.iter(|| {
        is_odd_mutrec(val);
        val += 1;
    });
}

benchmark_group!(
    benches,
    bench_oddness_loop,
    bench_oddness_rec,
    bench_oddness_res_rec,
    bench_oddness_boom,
    bench_oddness_mutrec
);

benchmark_main!(benches);
