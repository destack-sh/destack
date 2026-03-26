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

/// Exceptional direct calls branch to the normal continuation on success.
#[test]
fn test_exceptional_call_branches_to_normal_target() {
    let mir = r#"
function @double(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iadd v0, v0
    return v1
}

function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 10i32
    call @double(v0) normal block1(v1) unwind block2
block1(v2: i32, v3: i32):
    v4: i32 = iadd v2, v3
    return v4
block2(v5: ref<managed readonly void>):
    v6: i32 = iconst 0i32
    return v6
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(8)], Value::int32(26));
}

/// Exceptional direct calls branch to the unwind continuation on throw.
#[test]
fn test_exceptional_call_branches_to_unwind_target() {
    let mir = r#"
type @Error = { code: i32 }

function @thrower() -> i32 {
block0:
    v0: i32 = iconst 1i32
    v1: @Error = struct @Error (v0)
    v2: ref<managed readonly @Error> = managed.alloc @Error
    store v2, v1
    throw v2
}

function @caller() -> i32 {
block0:
    call @thrower() normal block1 unwind block2
block1(v0: i32):
    return v0
block2(v1: ref<managed readonly @Error>):
    v2: i32 = iconst 9i32
    return v2
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(9));
}

/// Thrown exceptions skip plain call resumes until one unwind continuation handles them.
#[test]
fn test_exceptional_call_unwinds_past_plain_call_resume() {
    let mir = r#"
type @Error = { code: i32 }

function @thrower() -> i32 {
block0:
    v0: i32 = iconst 1i32
    v1: @Error = struct @Error (v0)
    v2: ref<managed readonly @Error> = managed.alloc @Error
    store v2, v1
    throw v2
}

function @middle() -> i32 {
block0:
    v0: i32 = call @thrower() -> fn() -> i32
    return v0
}

function @caller() -> i32 {
block0:
    call @middle() normal block1 unwind block2
block1(v0: i32):
    return v0
block2(v1: ref<managed readonly @Error>):
    v2: i32 = iconst 13i32
    return v2
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(13));
}
