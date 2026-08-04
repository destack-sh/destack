use crate::tests::TestProgram;

/// Transfer values through one stable frame local.
#[test]
fn test_emit_bytecode_local_memory() {
    let program = TestProgram::mir(
        r#"
export function roundtrip(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    store v2, v1
    v3: int32 = load v2
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function roundtrip {
    move r2, r0
    move r0, r2
    frame.address r1, r2
    store r1, r0: int32
    load r0, r1: int32
    return r0
}
"#,
    );
}

/// Materialize constant, local, and shared global addresses.
#[test]
fn test_emit_bytecode_global_memory() {
    let program = TestProgram::mir(
        r#"
constant constantValue: int32 = 1
global localValue: int32 = zeroInit
shared global sharedValue: int32 = zeroInit

export function globals(v0: int32): int32 {
entry(v0: int32):
    v1: ref<int32, borrowed, readonly, constant> = global.address constantValue
    v2: ref<int32, borrowed, mutable, global> = global.address localValue
    v3: ref<int32, borrowed, mutable, shared global> = global.address sharedValue
    v4: int32 = load v1
    store v2, v0
    store v3, v4
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function globals {
    global.address.constant r1, g0
    global.address.local r2, g1
    global.address.shared r3, g2
    load.constant r4, r1: int32
    store r2, r0: int32
    store r3, r4: int32
    return r4
}
"#,
    );
}

/// Materialize local and shared heap references from one memory map.
#[test]
fn test_emit_bytecode_heap_memory() {
    let program = TestProgram::mir(
        r#"
export function transfer(
    v0: ref<int32, borrowed, readonly>,
    v1: ref<int32, borrowed, mutable, shared>,
): int32 {
entry(v0: ref<int32, borrowed, readonly>, v1: ref<int32, borrowed, mutable, shared>):
    v2: int32 = load v0
    store v1, v2
    return v2
}
"#,
    );

    program.assert_bytecode(
        r#"
function transfer {
    load r2, r0: int32
    store r1, r2: int32
    return r2
}
"#,
    );
}
