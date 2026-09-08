use crate::tests::TestProgram;

/// Transfer values through one stable frame local.
#[test]
fn test_emit_local_memory() {
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
    store.int32 r1, r0
    load.int32 r0, r1
    return r0
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
fn test_emit_global_memory() {
    let program = TestProgram::mir(
        r#"
constant constantValue: int32 = 1
global localValue: int32 = zeroinit
shared global sharedValue: int32 = zeroinit

export function globals(v0: int32): int32 {
entry(v0: int32):
    v1: ref<int32, borrowed, readonly, constant> = global.address constantValue
    v2: ref<int32, borrowed, mutable, static> = global.address localValue
    v3: ref<int32, borrowed, mutable, shared static> = global.address sharedValue
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
    global.address r1, g0
    global.address r2, g1
    global.address r3, g2
    load.int32 r4, r1
    store.int32 r2, r0
    store.int32 r3, r4
    return r4
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
    v4 = load.i64 notrap aligned v0+48
    v5 = iadd v4, v3
    v6 = symbol_value.i64 gv1
    v7 = load.i64 notrap aligned v6
    v8 = load.i64 notrap aligned v0+80
    v9 = iadd v8, v7
    v10 = symbol_value.i64 gv2
    v11 = load.i64 notrap aligned v10
    v12 = load.i64 notrap aligned v0+64
    v13 = iadd v12, v11
    v14 = load.i64 notrap aligned v0+40
    v15 = iadd v14, v5
    v16 = load.i32 notrap aligned v15
    v17 = load.i64 notrap aligned v0+40
    v18 = iadd v17, v9
    store notrap aligned v1, v18
    v19 = load.i64 notrap aligned v0+40
    v20 = iadd v19, v13
    store notrap aligned v16, v20
    return v16
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

/// Materialize local and shared heap references from one memory map.
#[test]
fn test_emit_heap_memory() {
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
    load.int32 r2, r0
    store.int32 r1, r2
    return r2
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

/// Report one changed reference range.
#[test]
fn test_emit_a_barrier() {
    let program = TestProgram::mir(
        r#"
export function update(v0: ref<int32, managed, mutable>): void {
entry(v0: ref<int32, managed, mutable>):
    v1: usize = 0
    v2: usize = 8
    barrier.write v0, v1, v2
    return
}
"#,
    );

    program.assert_bytecode(
        r#"
function update {
    constant.uint64 r1, 0
    constant.uint64 r2, 8
    barrier r0, r1, r2: ref<managed, local>
    return
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64) native {
    sig0 = (i64, i32, i64, i64, i64) native

block0(v0: i64, v1: i64):
    v2 = iconst.i64 0
    v3 = iconst.i64 8
    v4 = iconst.i32 0
    v5 = load.i64 notrap aligned v0+8
    v6 = load.i64 notrap aligned v5+32
    call_indirect sig0, v6(v0, v4, v1, v2, v3)  ; v4 = 0, v2 = 0, v3 = 8
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

/// Address one structural field in frame and referenced storage.
#[test]
fn test_emit_field_address() {
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

export function secondReference(v0: ref<Triple, borrowed, readonly>): int64 {
entry(v0: ref<Triple, borrowed, readonly>):
    v1: ref<int64, borrowed, readonly> = field.address v0, 1
    v2: int64 = load v1
    return v2
}
"#,
    );

    program.assert_bytecode(
        r#"
function second {
    frame.address r3, r0:r2
    address.add r3, r3, 8
    load.int64 r0, r3
    return r0
}

function secondReference {
    move r1, r0
    address.add r1, r1, 8
    load.int64 r0, r1
    return r0
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

function u0:2(i64, i64) -> i64 native {
block0(v0: i64, v1: i64):
    v2 = iconst.i64 8
    v3 = iadd v1, v2  ; v2 = 8
    v4 = load.i64 notrap aligned v0+40
    v5 = iadd v4, v3
    v6 = load.i64 notrap aligned v5
    return v6
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64, i64) -> i64 native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}

/// Address one fixed array element chosen at runtime, in inline and referenced storage.
#[test]
fn test_emit_fixed_array_address() {
    let program = TestProgram::mir(
        r#"
export function select(v0: [int32; 3], v1: usize): int32 {
entry(v0: [int32; 3], v1: usize):
    v2: ref<int32, borrowed, mutable, frame> = element.address v0, v1
    v3: int32 = load v2
    return v3
}

export function selectReference(v0: ref<[int32; 3], borrowed, readonly>, v1: usize): int32 {
entry(v0: ref<[int32; 3], borrowed, readonly>, v1: usize):
    v2: ref<int32, borrowed, readonly> = element.address v0, v1
    v3: int32 = load v2
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function select {
    frame.address r3, r0:r1
    address.add r3, r3, r2, 4
    load.int32 r0, r3
    return r0
}

function selectReference {
    move r2, r0
    address.add r2, r2, r1, 4
    load.int32 r0, r2
    return r0
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

function u0:2(i64, i64, i64) -> i32 native {
block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 4
    v4 = imul v2, v3  ; v3 = 4
    v5 = iadd v1, v4
    v6 = load.i64 notrap aligned v0+40
    v7 = iadd v6, v5
    v8 = load.i32 notrap aligned v7
    return v8
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64, i64, i64) -> i32 native
    fn0 = colocated u0:2 sig0

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

/// Advance one stable slice reference before materializing its native pointer.
#[test]
fn test_emit_slice_address() {
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

    program.assert_bytecode(
        r#"
function select {
    move r3, r0
    address.add r3, r3, r2, 4
    load.int32 r0, r3
    return r0
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
