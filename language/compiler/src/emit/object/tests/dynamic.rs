use crate::tests::TestProgram;

/// Emit dynamic binding and direct descriptor projections.
#[test]
fn test_emit_dynamic() {
    let mut program = TestProgram::mir(
        r#"
type Writer { }
type FileWriter { }

export function inspect(v0: ref<FileWriter, managed, readonly, local>): typeId {
entry(v0: ref<FileWriter, managed, readonly, local>):
    v1: dynamic<Writer, managed, readonly, local> = dynamic.bind v0, FileWriter
    v2: ref<void, managed, readonly, local> = dynamic.payload v1
    v3: typeId = dynamic.type v1
    return v3
}
"#,
    );
    program.mark_dynamic("FileWriter", "Writer");

    program.assert_bytecode(
        r#"function inspect {
    dynamic.bind r1:r2, r0, d0
    move r0, r1
    dynamic.type r0, r1:r2
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = symbol_value.i64 gv2
    v3 = load.i32 notrap aligned v2
    v4 = load.i64 notrap aligned region0 v0+32
    v5 = uextend.i64 v3
    v6 = iconst.i64 3
    v7 = ishl v5, v6  ; v6 = 3
    v8 = iadd v4, v7
    v9 = load.i64 notrap aligned v8
    v10 = load.i32 notrap aligned v9
    return v10
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}

/// Read one structural field through its linked dynamic dispatch slot.
#[test]
fn test_emit_dynamic_read() {
    let mut program = TestProgram::mir(
        r#"
type Writer { value: int32 }
type FileWriter { value: int32 }

export function read(v0: dynamic<Writer, managed, readonly, local>): int32 {
entry(v0: dynamic<Writer, managed, readonly, local>):
    v1: int32 = dynamic.read v0, 0
    return v1
}
"#,
    );
    program.mark_dynamic("FileWriter", "Writer");

    program.assert_bytecode(
        r#"function read {
    dynamic.read r2, r0:r1[0], 4
    return r2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    v3 = load.i64 notrap aligned region0 v0+32
    v4 = uextend.i64 v2
    v5 = iconst.i64 3
    v6 = ishl v4, v5  ; v5 = 3
    v7 = iadd v3, v6
    v8 = load.i64 notrap aligned v7
    v9 = load.i32 notrap aligned v8+4
    v10 = load.i64 notrap aligned region0 v0+40
    v11 = iadd v10, v1
    v12 = uextend.i64 v9
    v13 = iadd v11, v12
    v14 = load.i32 notrap aligned region1 v13
    return v14
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) -> i32 native
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
