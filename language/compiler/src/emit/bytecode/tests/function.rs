use destack_bytecode as bytecode;

use crate::tests::TestProgram;

/// Emit scalar and multiword function registers into canonical bytecode text.
#[test]
fn test_emit_bytecode_function() {
    let program = TestProgram::mir(
        r#"
export function add(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

export function identity(v0: slice<int32, managed, mutable>): slice<int32, managed, mutable> {
entry(v0: slice<int32, managed, mutable>):
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function add(r0: t1, r1: t1): t1 {
    int.add.int32 r2, r0, r1
    return r2
}

function identity(r0:r1: t2): t2 {
    return r0:r1
}
"#,
    );
}

/// Pack scattered MIR values into one contiguous outgoing call window.
#[test]
fn test_emit_call_arguments() {
    let program = TestProgram::mir(
        r#"
external function consume(int32, boolean, int32): int32

export function caller(v0: int32, v1: boolean, v2: int32): int32 {
entry(v0: int32, v1: boolean, v2: int32):
    v3: int32 = int.add v0, v2
    v4: int32 = call consume(v3, v1, v0): (int32, boolean, int32) => int32
    return v4
}
"#,
    );

    let bytecode = program.assert_bytecode(
        r#"
function consume(r0: t1, r1: t2, r2: t1): t1

function caller(r0: t1, r1: t2, r2: t1): t1 {
    int.add.int32 r3, r0, r2
    move r4, r3
    move r5, r1
    move r6, r0
    call r2, consume, r4:r6
    return r2
}
"#,
    );

    // retain the semantic call coordinate after its outgoing register moves
    let function = bytecode::FunctionId(1);
    let instruction = bytecode
        .operation(function, 1)
        .expect("caller operation should decode")
        .expect("caller operation should exist");

    assert_eq!(instruction.opcode(), bytecode::Opcode::CALL);
}
