use crate::tests::TestProgram;

/// Emit one allocation-selected destructor call for an erased unique value.
#[test]
fn test_emit_indirect_drop() {
    let program = TestProgram::mir(
        r#"
type Writer { }

export function release(v0: dynamic<Writer, unique, mutable, local>): void {
entry(v0: dynamic<Writer, unique, mutable, local>):
    drop v0
    return
}

export function releaseFunction(v0: function<() => void, once, unique, mutable, local>): void {
entry(v0: function<() => void, once, unique, mutable, local>):
    drop v0
    return
}
"#,
    );

    program.assert_bytecode(
        r#"function release {
    drop r0
    return
}

function releaseFunction {
    drop r1
    return
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i32) native {
    ss0 = explicit_slot 1, key = 0
    ss1 = explicit_slot 16, align = 8, key = 1
    gv0 = symbol colocated userextname0
    sig0 = (i64, i64, i32, i64) native

block0(v0: i64, v1: i64, v2: i32):
    v3 = stack_addr.i64 ss0
    v4 = stack_addr.i64 ss1
    store notrap aligned v1, v4
    store notrap aligned v2, v4+8
    v5 = symbol_value.i64 gv0
    v6 = load.i32 notrap aligned v5
    v7 = load.i64 notrap aligned v0+8
    v8 = load.i64 notrap aligned v7+16
    call_indirect sig0, v8(v0, v1, v6, v3), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    return
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i32) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    call fn0(v0, v3, v4)
    return
}

function u0:2(i64, i64, i64) native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 16, align = 8, key = 4294967297
    gv0 = symbol colocated userextname0
    sig0 = (i64, i64, i32, i64) native

block0(v0: i64, v1: i64, v2: i64):
    v3 = stack_addr.i64 ss0
    v4 = stack_addr.i64 ss1
    store notrap aligned v1, v4
    store notrap aligned v2, v4+8
    v5 = symbol_value.i64 gv0
    v6 = load.i32 notrap aligned v5
    v7 = load.i64 notrap aligned v0+8
    v8 = load.i64 notrap aligned v7+16
    call_indirect sig0, v8(v0, v2, v6, v3), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    return
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64, i64, i64) native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    call fn0(v0, v3, v4)
    return
}
"#,
    );
}

/// Call one generated frame destructor with an exclusive frame reference.
#[test]
fn test_emit_drop() {
    let mut program = TestProgram::mir(
        r#"
type Resource {
    value: int32;
}

function destroy<'a>(v0: ref<Resource, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<Resource, borrowed, 'a, mutable, frame>):
    return
}

export function release(v0: Resource): void {
entry(v0: Resource):
    drop v0
    return
}
"#,
    );
    program.mark_destructor("Resource", "destroy");

    program.assert_bytecode(
        r#"
function destroy {
    return
}

function release {
    drop r0, destroy
    return
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64) native {
block0(v0: i64, v1: i64):
    return
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    call fn0(v0, v3)
    return
}

function u0:2(i64, i32) native {
    ss0 = explicit_slot 4, align = 4
    sig0 = (i64, i64) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    store notrap aligned v1, v2
    call fn0(v0, v2)
    return
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
