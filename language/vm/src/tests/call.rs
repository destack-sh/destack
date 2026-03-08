use crate::tests::run_mir_expect;
use destack_heap::Value;

/// function.addr produces a callable pointer for call.indirect.
#[test]
fn test_function_addr_indirect_call() {
    let mir = r#"
function @add(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iadd v0, v0
    return v1
}

function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1: fn(i32) -> i32 = function.addr @add
    v2: i32 = call.indirect v1(v0) -> fn(i32) -> i32
    return v2
}"#;
    run_mir_expect(mir, "caller", &[Value::int32(21)], Value::int32(42));
}
