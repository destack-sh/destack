use crate::tests::TestProgram;

/// Call direct waiter and task runtime operations with canonical values.
#[test]
fn test_emit_native_scheduler_operations() {
    let program = TestProgram::mir(
        r#"
type Task {
    uint64;
}

export function schedule(v0: waiter<int32>, v1: int32): Task {
entry(v0: waiter<int32>, v1: int32):
    v2: boolean = waiter.queue v0, v1
    v3: boolean = waiter.cancel v0
    v4: Task = task.resolve v1
    task.park v4, v0
    task.cancel v4
    task.detach v4
    return v4
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i32) -> i64 native {
    ss0 = explicit_slot 8, align = 8
    ss1 = explicit_slot 8, align = 8
    gv0 = symbol colocated userextname0
    gv1 = symbol colocated userextname0
    sig0 = (i64, i64, i32, i64) -> i32 native
    sig1 = (i64, i64) -> i32 native
    sig2 = (i64, i32, i64) -> i64 native
    sig3 = (i64, i64, i64) native
    sig4 = (i64, i64) native
    sig5 = (i64, i64) native

block0(v0: i64, v1: i64, v2: i32):
    v3 = stack_addr.i64 ss0
    v4 = sextend.i64 v2
    store notrap aligned v4, v3
    v5 = symbol_value.i64 gv0
    v6 = load.i32 notrap aligned v5
    v7 = load.i64 notrap aligned v0+8
    v8 = load.i64 notrap aligned v7+104
    v9 = call_indirect sig0, v8(v0, v1, v6, v3)
    v10 = ireduce.i8 v9
    v11 = load.i64 notrap aligned v0+8
    v12 = load.i64 notrap aligned v11+112
    v13 = call_indirect sig1, v12(v0, v1)
    v14 = ireduce.i8 v13
    v15 = stack_addr.i64 ss1
    v16 = sextend.i64 v2
    store notrap aligned v16, v15
    v17 = symbol_value.i64 gv1
    v18 = load.i32 notrap aligned v17
    v19 = load.i64 notrap aligned v0+8
    v20 = load.i64 notrap aligned v19+120
    v21 = call_indirect sig2, v20(v0, v18, v15)
    v22 = load.i64 notrap aligned v0+8
    v23 = load.i64 notrap aligned v22+144
    call_indirect sig3, v23(v0, v21, v1)
    v24 = load.i64 notrap aligned v0+8
    v25 = load.i64 notrap aligned v24+152
    call_indirect sig4, v25(v0, v21)
    v26 = load.i64 notrap aligned v0+8
    v27 = load.i64 notrap aligned v26+168
    call_indirect sig5, v27(v0, v21)
    return v21
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i32) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Deoptimize continuation control at exact reconstructable Program operations.
#[test]
fn test_emit_native_continuation_control() {
    let program = TestProgram::mir(
        r#"
function* generate(v0: int32): int32 {
entry(v0: int32):
    yield v0 => resumed | completed | unwind

resumed(v1: int32):
    return v1

completed(v2: int32):
    return v2

unwind:
    unwind.resume
}

export function owner(v0: int32): void {
entry(v0: int32):
    v1: continuation<int32, int32, int32> = continuation.new generate(v0)
    continuation.destroy v1
    return
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i32) -> i32 native {
    ss0 = explicit_slot 1, key = 0
    ss1 = explicit_slot 4, align = 4, key = 1
    gv0 = symbol colocated userextname0
    sig0 = (i64, i32, i64) native

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    v3 = stack_addr.i64 ss1
    store notrap aligned v1, v3
    v4 = symbol_value.i64 gv0
    v5 = load.i32 notrap aligned v4
    v6 = load.i64 notrap aligned v0+8
    v7 = load.i64 notrap aligned v6+64
    call_indirect sig0, v7(v0, v5, v2), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    trap user4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}

function u0:2(i64, i32) native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 4, align = 4, key = 4294967297
    gv0 = symbol colocated userextname0
    sig0 = (i64, i32, i64) native

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    v3 = stack_addr.i64 ss1
    store notrap aligned v1, v3
    v4 = symbol_value.i64 gv0
    v5 = load.i32 notrap aligned v4
    v6 = load.i64 notrap aligned v0+8
    v7 = load.i64 notrap aligned v6+64
    call_indirect sig0, v7(v0, v5, v2), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    trap user4
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64, i32) native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    call fn0(v0, v3)
    return
}
"#,
    );
}
