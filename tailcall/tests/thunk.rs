use tailcall::thunk::Thunk;

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
