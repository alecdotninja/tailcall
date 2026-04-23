use tailcall::{runtime::Thunk, tailcall};

#[tailcall]
pub fn countdown(input: u32) -> u32 {
    if input == 0 {
        0
    } else {
        tailcall::call! { countdown(input - 1) }
    }
}

#[tailcall]
pub fn is_even(input: u32) -> bool {
    if input == 0 {
        true
    } else {
        tailcall::call! { is_odd(input - 1) }
    }
}

#[tailcall]
pub fn is_odd(input: u32) -> bool {
    if input == 0 {
        false
    } else {
        tailcall::call! { is_even(input - 1) }
    }
}

pub fn runtime_countdown(input: u32) -> u32 {
    build_countdown(input).call()
}

fn build_countdown(input: u32) -> Thunk<'static, u32> {
    Thunk::bounce(move || {
        if input == 0 {
            Thunk::value(0)
        } else {
            build_countdown(input - 1)
        }
    })
}
