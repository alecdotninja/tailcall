#![no_std]

use tailcall::{tailcall, Thunk};

#[tailcall]
pub fn countdown(input: u32) -> u32 {
    if input == 0 {
        0
    } else {
        tailcall::call! { countdown(input - 1) }
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
