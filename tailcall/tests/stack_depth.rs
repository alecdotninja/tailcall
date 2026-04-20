use backtrace::Backtrace;
use tailcall::*;

#[tailcall]
fn tail_recurse_then_call<Callback: FnOnce()>(times: usize, callback: Callback) {
    if times > 0 {
        tailcall::call! { tail_recurse_then_call(times - 1, callback) }
    } else {
        callback()
    }
}

fn current_stack_depth() -> usize {
    Backtrace::new_unresolved().frames().len()
}

#[test]
fn test_tail_recusive_calls_do_not_result_in_new_stack_frames() {
    let stack_depth_before_recursion = current_stack_depth();

    tail_recurse_then_call(100, move || {
        let stack_depth_after_recursion = current_stack_depth();
        let change_in_stack_depth = stack_depth_after_recursion - stack_depth_before_recursion;

        // The thunk runtime adds a few dispatch frames compared to the old loop state machine,
        // but the recursion depth itself should still stay constant.
        assert!(change_in_stack_depth <= 6);
    });
}
