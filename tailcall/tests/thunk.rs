use tailcall::thunk::Thunk;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

#[test]
fn sanity() {
    let thunk = Thunk::new(|| 42);
    assert_eq!(thunk.call(), 42);
}

#[test]
fn with_captures() {
    let x = 42;
    let y = 25;

    let thunk = Thunk::new(move || x + y);

    assert_eq!(thunk.call(), x + y);
}

#[test]
#[should_panic]
fn with_too_many_captures() {
    let a = 1usize;
    let b = 2usize;
    let c = 3usize;
    let d = 4usize;
    let e = 5usize;
    let f = 6usize;
    let g = 7usize;
    let h = 8usize;

    Thunk::new(move || a + b + c + d + e + f + g + h);
}

#[test]
fn dropping_without_call_runs_destructor_once() {
    let drops = Rc::new(AtomicUsize::new(0));
    let captured = DropCounter(drops.clone());

    let thunk = Thunk::new(move || {
        let _captured = captured;
        42
    });

    drop(thunk);

    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn calling_runs_destructor_once() {
    let drops = Rc::new(AtomicUsize::new(0));
    let captured = DropCounter(drops.clone());

    let thunk = Thunk::new(move || {
        let _captured = captured;
        42
    });

    assert_eq!(thunk.call(), 42);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn panic_during_call_drops_capture_once() {
    let drops = Rc::new(AtomicUsize::new(0));
    let captured = DropCounter(drops.clone());

    let thunk = Thunk::new(move || {
        let _captured = captured;
        panic!("boom");
    });

    let _ = catch_unwind(AssertUnwindSafe(|| thunk.call()));

    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

struct DropCounter(Rc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
