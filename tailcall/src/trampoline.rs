use crate::slot::Slot;
use crate::thunk::Thunk;

pub enum Action<'slot, T> {
    Done(T),
    Call(Thunk<'slot, Self>),
}

#[inline(always)]
pub fn done<T>(_slot: &mut Slot, value: T) -> Action<T> {
    Action::Done(value)
}

#[inline(always)]
pub fn call<'slot, T, F>(slot: &'slot mut Slot, fn_once: F) -> Action<'slot, T>
where
    F: FnOnce(&'slot mut Slot) -> Action<'slot, T> + 'slot,
{
    Action::Call(Thunk::new_in(slot, fn_once))
}

#[inline(always)]
pub fn run<T>(build_action: impl FnOnce(&mut Slot) -> Action<T>) -> T {
    let slot = &mut Slot::new();

    let mut action = build_action(slot);

    loop {
        match action {
            Action::Done(value) => return value,
            Action::Call(thunk) => action = thunk.call(),
        }
    }
}
