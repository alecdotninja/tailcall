use crate::thunk::Thunk;

pub enum Action<'a, T> {
    Done(T),
    Call(Thunk<'a, Self>),
}

pub const fn done<'a, T>(value: T) -> Action<'a, T> {
    Action::Done(value)
}

pub const fn call<'a, T, F>(fn_once: F) -> Action<'a, T>
where
    F: FnOnce() -> Action<'a, T> + 'a,
{
    Action::Call(Thunk::new(fn_once))
}

pub fn run<T>(mut action: Action<'_, T>) -> T {
    loop {
        match action {
            Action::Call(thunk) => action = thunk.call(),
            Action::Done(value) => return value,
        }
    }
}
