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

@environment(ref<void, managed, mutable>)
export function closure(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function add {
    int.add r2, r0, r1: int32
    return r2
}

function identity {
    return r0:r1
}

function closure {
    return r1
}
"#,
    );
}

/// Emit function pointers, captured environments, and environment projection.
#[test]
fn test_emit_bytecode_function_value() {
    let program = TestProgram::mir(
        r#"
function increment(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = int.add v0, v1
    return v2
}

@environment(ref<void, managed, mutable>)
function captured(v0: int32): int32 {
entry(v0: int32):
    return v0
}

export function address(): fn(int32) => int32 {
entry:
    v0: fn(int32) => int32 = function.address increment
    return v0
}

@environment(ref<void, managed, mutable>)
export function environment(
    v0: ref<void, managed, mutable>,
): ref<void, managed, mutable> {
entry(v0: ref<void, managed, mutable>):
    v1: ref<void, managed, mutable> = function.environment.current
    v2: function<(int32) => int32, repeatable, managed, mutable> = function.bind captured, v0
    v3: ref<void, managed, mutable> = function.environment v2
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function increment {
    constant r1, 1: int32
    int.add r2, r0, r1: int32
    return r2
}

function captured {
    return r1
}

function address {
    function.address r0, increment
    return r0
}

function environment {
    move r2, r0
    function.bind r3:r4, captured, r1
    extract r1, r3:r4, 8:8
    return r1
}
"#,
    );
}
