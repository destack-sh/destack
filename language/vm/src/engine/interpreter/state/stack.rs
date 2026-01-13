use crate::memory::Value;

/// Resize a stack and clear the active range.
#[allow(clippy::uninit_vec)]
pub(crate) fn resize_and_clear_stack(stack: &mut Vec<Value>, base: usize, end: usize) {
    // validate bounds
    debug_assert!(base <= end, "stack range out of bounds: {base}..{end}");

    // resize without redundant initialization
    if end > stack.len() {
        let additional = end - stack.len();
        stack.reserve(additional);

        // safety: fill the new range immediately
        unsafe {
            stack.set_len(end);
        }
    } else {
        stack.truncate(end);
    }

    // clear active stack slots
    stack[base..end].fill(Value::VOID);
}
