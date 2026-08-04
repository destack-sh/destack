use crate::tests::TestProgram;

/// Transport block arguments through conditional edges.
#[test]
fn test_emit_bytecode_control_flow() {
    let program = TestProgram::mir(
        r#"
export function select(v0: boolean, v1: int32, v2: int32): int32 {
entry(v0: boolean, v1: int32, v2: int32):
    branch v0 => selected(v1) | selected(v2)

selected(v3: int32):
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function select {
    branch r0 => b1 | b2

b0:
    return r0

b1:
    move r0, r1
    jump b0

b2:
    move r0, r2
    jump b0
}
"#,
    );
}

/// Emit compact switch dispatch and edge argument transfers.
#[test]
fn test_emit_bytecode_switch() {
    let program = TestProgram::mir(
        r#"
export function dispatch(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    switch v0, fallback(v2), 0 => selected(v1), 17 => selected(v2)

selected(v3: int32):
    return v3

fallback(v4: int32):
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function dispatch {
    switch r0 { 0 => b2, 17 => b3, default => b4 }

b0:
    return r0

b1:
    return r0

b2:
    move r0, r1
    jump b0

b3:
    move r0, r2
    jump b0

b4:
    move r0, r2
    jump b1
}
"#,
    );
}

/// Emit explicit scalar checks with normal and failure continuations.
#[test]
fn test_emit_bytecode_checks() {
    let program = TestProgram::mir(
        r#"
export function guard(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    check bounds.s v0, v1, v0 => valid | invalid

valid:
    check int.add.overflow.s v0, v1 => result | invalid

result:
    return v0

invalid:
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function guard {
    check.bounds r0, r1: int32 | b2
    jump b0

b0:
    check.add.overflow r0, r1: int32 | b2
    jump b1

b1:
    return r0

b2:
    return r1
}
"#,
    );
}

/// Emit unreachable control flow as one terminal bytecode operation.
#[test]
fn test_emit_bytecode_unreachable() {
    let program = TestProgram::mir(
        r#"
export function fail(): never {
entry:
    unreachable
}
"#,
    );

    program.assert_bytecode(
        r#"
function fail {
    unreachable
}
"#,
    );
}
