use crate::tests::TestProgram;

/// Read, write, and address one canonical native local.
#[test]
fn test_emit_native_local_memory() {
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

    program.assert_native(
        r#"
function u0:0(i64, i32) -> i32 native {
    ss0 = explicit_slot 4, align = 4, key = 0

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    store notrap aligned v1, v2
    v3 = load.i32 notrap aligned v2
    store notrap aligned v3, v2
    v4 = load.i32 notrap aligned v2
    return v4
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
"#,
    );
}

/// Materialize constant, local, and shared global addresses.
#[test]
fn test_emit_native_global_memory() {
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

    program.assert_native(
        r#"
function u0:0(i64, i32) -> i32 native {
    gv0 = symbol colocated userextname0
    gv1 = symbol colocated userextname1
    gv2 = symbol colocated userextname2

block0(v0: i64, v1: i32):
    v2 = symbol_value.i64 gv0
    v3 = load.i64 notrap aligned v2
    v4 = symbol_value.i64 gv1
    v5 = load.i64 notrap aligned v4
    v6 = load.i64 notrap aligned v0+96
    v7 = iadd v6, v5
    v8 = symbol_value.i64 gv2
    v9 = load.i64 notrap aligned v8
    v10 = load.i64 notrap aligned v0+80
    v11 = iadd v10, v9
    v12 = load.i64 notrap aligned v0+48
    v13 = iadd v12, v3
    v14 = load.i32 notrap aligned v13
    v15 = load.i64 notrap aligned v0+40
    v16 = iadd v15, v7
    store notrap aligned v1, v16
    v17 = load.i64 notrap aligned v0+40
    v18 = iadd v17, v11
    store notrap aligned v14, v18
    return v14
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
"#,
    );
}

/// Materialize local and shared heap references from the same MemoryMap base.
#[test]
fn test_emit_native_heap_memory() {
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

    program.assert_native(
        r#"
function u0:0(i64, i64, i64) -> i32 native {
block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v0+40
    v4 = iadd v3, v1
    v5 = load.i32 notrap aligned v4
    v6 = load.i64 notrap aligned v0+40
    v7 = iadd v6, v2
    store notrap aligned v5, v7
    return v5
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Pin one managed allocation and report one changed reference range.
#[test]
fn test_emit_native_pin_and_barrier() {
    let program = TestProgram::mir(
        r#"
export function update(v0: ref<int32, managed, mutable>): void {
entry(v0: ref<int32, managed, mutable>):
    v1: ref<int32, managed, mutable> = pin v0
    v2: usize = 0
    v3: usize = 8
    barrier.write v0, v2, v3
    unpin v1
    return
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64) native {
    sig0 = (i64, i32, i64) -> i64 native
    sig1 = (i64, i32, i64, i64, i64) native
    sig2 = (i64, i32, i64) native

block0(v0: i64, v1: i64):
    v2 = iconst.i32 0
    v3 = load.i64 notrap aligned v0+8
    v4 = load.i64 notrap aligned v3+24
    v5 = call_indirect sig0, v4(v0, v2, v1)  ; v2 = 0
    v6 = iconst.i64 0
    v7 = iconst.i64 8
    v8 = iconst.i32 0
    v9 = load.i64 notrap aligned v0+8
    v10 = load.i64 notrap aligned v9+40
    call_indirect sig1, v10(v0, v8, v1, v6, v7)  ; v8 = 0, v6 = 0, v7 = 8
    v11 = iconst.i32 0
    v12 = load.i64 notrap aligned v0+8
    v13 = load.i64 notrap aligned v12+32
    call_indirect sig2, v13(v0, v11, v5)  ; v11 = 0
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
"#,
    );
}

/// Address one structural field in canonical frame storage.
#[test]
fn test_emit_native_field_address() {
    let program = TestProgram::mir(
        r#"
type Triple {
    first: int64;
    second: int64;
    third: int64;
}

export function second(v0: Triple): int64 {
entry(v0: Triple):
    v1: ref<int64, borrowed, readonly, frame> = field.address v0, 1
    v2: int64 = load v1
    return v2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64) -> i64 native {
block0(v0: i64, v1: i64):
    v2 = iconst.i64 8
    v3 = iadd v1, v2  ; v2 = 8
    v4 = load.i64 notrap aligned v3
    return v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 0
    v4 = iadd v1, v3  ; v3 = 0
    v5 = call fn0(v0, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Address one runtime-selected fixed-array element in canonical frame storage.
#[test]
fn test_emit_native_fixed_array_address() {
    let program = TestProgram::mir(
        r#"
export function select(v0: [int32; 3], v1: usize): int32 {
entry(v0: [int32; 3], v1: usize):
    v2: ref<int32, borrowed, mutable, frame> = element.address v0, v1
    v3: int32 = load v2
    return v3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i64) -> i32 native {
block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 4
    v4 = imul v2, v3  ; v3 = 4
    v5 = iadd v1, v4
    v6 = load.i32 notrap aligned v5
    return v6
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 0
    v4 = iadd v1, v3  ; v3 = 0
    v5 = load.i64 notrap aligned v1+16
    v6 = call fn0(v0, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Advance one stable slice reference before materializing its native pointer.
#[test]
fn test_emit_native_slice_address() {
    let program = TestProgram::mir(
        r#"
export function select(v0: slice<int32, borrowed, readonly>, v1: usize): int32 {
entry(v0: slice<int32, borrowed, readonly>, v1: usize):
    v2: ref<int32, borrowed, readonly> = element.address v0, v1
    v3: int32 = load v2
    return v3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i64, i64) -> i32 native {
block0(v0: i64, v1: i64, v2: i64, v3: i64):
    v4 = iconst.i64 4
    v5 = imul v3, v4  ; v4 = 4
    v6 = iadd v1, v5
    v7 = load.i64 notrap aligned v0+40
    v8 = iadd v7, v6
    v9 = load.i32 notrap aligned v8
    return v9
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64, i64) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i64 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}
