use backtrace::Backtrace;
use tailcall::*;

#[tailcall]
fn tail_recurse_then_call<Callback: FnOnce()>(times: usize, callback: Callback) {
    if times > 0 {
        tail_recurse_then_call(times - 1, callback)
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

        // There should be at most one frame for `tail_recurse_then_call` and one frame for the
        // callback itself; however, there _may_ be fewer if the compiler decides to inline.
        assert!(change_in_stack_depth <= 2);
    });
}
