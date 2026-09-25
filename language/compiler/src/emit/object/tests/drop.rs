use crate::tests::TestProgram;

/// Call one generated frame destructor with an exclusive frame reference.
#[test]
fn test_emit_drop() {
    let mut program = TestProgram::mir(
        r#"
type Resource {
    value: int32;
}

function destroy<'a>(v0: ref<Resource, borrowed, 'a, mutable>): void {
entry(v0: ref<Resource, borrowed, 'a, mutable>):
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
function u0:0(i64 vmctx, i64) native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64):
    return
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    call fn0(v0, v3)
    return
}

function u0:2(i64 vmctx, i32) native {
    ss0 = explicit_slot 4, align = 4
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    sig0 = (i64 vmctx, i64) native
    fn0 = colocated u0:0 sig0
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    store notrap aligned region1 v1, v2
    v3 = load.i64 notrap aligned region0 v0+40
    v4 = isub v2, v3
    call fn0(v0, v4)
    return
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    call fn0(v0, v3)
    return
}
"#,
    );
}
