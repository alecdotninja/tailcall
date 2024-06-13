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

// #[test]
// fn with_too_many_captures() {
//     let a = 1;
//     let b = 2;
//     let c = 3;
//     let d = 4;
//     let e = 5;
//     let f = 6;
//     let g = 7;

//     let thunk = Thunk::new(move || a + b + c + d + e + f + g);

//     assert_eq!(thunk.call(), a + b + c + d + e + f + g,);
// }
