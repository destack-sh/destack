use crate::tests::TestProgram;

/// Transfer values through one stable frame local.
#[test]
fn test_emit_local_memory() {
    let program = TestProgram::mir(
        r#"
export function roundtrip(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: ref<int32, borrowed, 'frame, mutable> = address l0
    store (*v2), v1
    v3: int32 = load (*v2)
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
function u0:0(i64 vmctx, i32) -> i32 native {
    ss0 = explicit_slot 4, align = 4, key = 0
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    store notrap aligned region1 v1, v2
    v3 = stack_addr.i64 ss0
    v4 = load.i32 notrap aligned region1 v3
    v5 = stack_addr.i64 ss0
    v6 = load.i64 notrap aligned region0 v0+40
    v7 = isub v5, v6
    v8 = load.i64 notrap aligned region0 v0+40
    v9 = iadd v8, v7
    store notrap aligned region1 v4, v9
    v10 = load.i64 notrap aligned region0 v0+40
    v11 = iadd v10, v7
    v12 = load.i32 notrap aligned region1 v11
    return v12
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) -> i32 native
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
    v1: ref<int32, borrowed, 'static, readonly> = address @constantValue
    v2: ref<int32, borrowed, 'static, mutable> = address @localValue
    v3: ref<int32, borrowed, 'static, mutable> = address @sharedValue
    v4: int32 = load (*v1)
    store (*v2), v0
    store (*v3), v4
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
function u0:0(i64 vmctx, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    gv3 = symbol colocated userextname1
    gv4 = symbol colocated userextname2
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = symbol_value.i64 gv2
    v3 = load.i64 notrap aligned v2
    v4 = load.i64 notrap aligned region0 v0+56
    v5 = iadd v4, v3
    v6 = symbol_value.i64 gv3
    v7 = load.i64 notrap aligned v6
    v8 = load.i64 notrap aligned region0 v0+88
    v9 = iadd v8, v7
    v10 = symbol_value.i64 gv4
    v11 = load.i64 notrap aligned v10
    v12 = load.i64 notrap aligned region0 v0+72
    v13 = iadd v12, v11
    v14 = load.i64 notrap aligned region0 v0+40
    v15 = iadd v14, v5
    v16 = load.i32 notrap aligned region1 v15
    v17 = load.i64 notrap aligned region0 v0+40
    v18 = iadd v17, v9
    store notrap aligned region1 v1, v18
    v19 = load.i64 notrap aligned region0 v0+40
    v20 = iadd v19, v13
    store notrap aligned region1 v16, v20
    return v16
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) -> i32 native
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
export function transfer<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, mutable>): int32 {
entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, mutable>):
    v2: int32 = load (*v0)
    store (*v1), v2
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
function u0:0(i64 vmctx, i64, i64) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned region0 v0+40
    v4 = iadd v3, v1
    v5 = load.i32 notrap aligned region1 v4
    v6 = load.i64 notrap aligned region0 v0+40
    v7 = iadd v6, v2
    store notrap aligned region1 v5, v7
    return v5
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64) -> i32 native
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
export function update(v0: ref<int32, managed, mutable, local>): void {
entry(v0: ref<int32, managed, mutable, local>):
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
    barrier r0, r1, r2
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
    sig0 = (i64, i64, i64, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i64 0
    v3 = iconst.i64 8
    v4 = load.i64 notrap aligned region0 v0+40
    v5 = iadd v4, v1
    v6 = load.i64 notrap aligned region0 v0+8
    v7 = load.i64 notrap aligned v6+32
    call_indirect sig0, v7(v0, v5, v2, v3)  ; v2 = 0, v3 = 8
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
    v1: ref<int64, borrowed, 'frame, readonly> = address (v0).1
    v2: int64 = load (*v1)
    return v2
}

export function secondReference<'a>(v0: ref<Triple, borrowed, 'a, readonly>): int64 {
entry(v0: ref<Triple, borrowed, 'a, readonly>):
    v1: ref<int64, borrowed, 'a, readonly> = address (*v0).1
    v2: int64 = load (*v1)
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
    address.add r1, r0, 8
    load.int64 r0, r1
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i64 8
    v3 = iadd v1, v2  ; v2 = 8
    v4 = load.i64 notrap aligned region0 v0+40
    v5 = isub v3, v4
    v6 = load.i64 notrap aligned region0 v0+40
    v7 = iadd v6, v5
    v8 = load.i64 notrap aligned region1 v7
    return v8
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 0
    v4 = iadd v1, v3  ; v3 = 0
    v5 = call fn0(v0, v4)
    store notrap aligned v5, v2
    return
}

function u0:2(i64 vmctx, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i64 8
    v3 = iadd v1, v2  ; v2 = 8
    v4 = load.i64 notrap aligned region0 v0+40
    v5 = iadd v4, v3
    v6 = load.i64 notrap aligned region1 v5
    return v6
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64 native
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
    v2: ref<int32, borrowed, 'frame, mutable> = address (v0)[v1]
    v3: int32 = load (*v2)
    return v3
}

export function selectReference<'a>(v0: ref<[int32; 3], borrowed, 'a, readonly>, v1: usize): int32 {
entry(v0: ref<[int32; 3], borrowed, 'a, readonly>, v1: usize):
    v2: ref<int32, borrowed, 'a, readonly> = address (*v0)[v1]
    v3: int32 = load (*v2)
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
    address.add r2, r0, r1, 4
    load.int32 r0, r2
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 4
    v4 = imul v2, v3  ; v3 = 4
    v5 = iadd v1, v4
    v6 = load.i64 notrap aligned region0 v0+40
    v7 = isub v5, v6
    v8 = load.i64 notrap aligned region0 v0+40
    v9 = iadd v8, v7
    v10 = load.i32 notrap aligned region1 v9
    return v10
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 0
    v4 = iadd v1, v3  ; v3 = 0
    v5 = load.i64 notrap aligned v1+16
    v6 = call fn0(v0, v4, v5)
    store notrap aligned v6, v2
    return
}

function u0:2(i64 vmctx, i64, i64) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 4
    v4 = imul v2, v3  ; v3 = 4
    v5 = iadd v1, v4
    v6 = load.i64 notrap aligned region0 v0+40
    v7 = iadd v6, v5
    v8 = load.i32 notrap aligned region1 v7
    return v8
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64) -> i32 native
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
export function select<'a>(v0: slice<int32, borrowed, 'a, readonly>, v1: usize): int32 {
entry(v0: slice<int32, borrowed, 'a, readonly>, v1: usize):
    v2: ref<int32, borrowed, 'a, readonly> = address (*v0)[v1]
    v3: int32 = load (*v2)
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function select {
    address.add r3, r0, r2, 4
    load.int32 r0, r3
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i64) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i64):
    v4 = iconst.i64 4
    v5 = imul v3, v4  ; v4 = 4
    v6 = iadd v1, v5
    v7 = load.i64 notrap aligned region0 v0+40
    v8 = iadd v7, v6
    v9 = load.i32 notrap aligned region1 v8
    return v9
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i64) -> i32 native
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

/// Address one variant payload through a reference.
#[test]
fn test_emit_variant_payload_address() {
    let program = TestProgram::mir(
        r#"
type Choice = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

export function payload<'a>(v0: ref<Choice, borrowed, 'a, readonly>): int64 {
entry(v0: ref<Choice, borrowed, 'a, readonly>):
    v1: ref<int64, borrowed, 'a, readonly> = address ((*v0) as 1)
    v2: int64 = load (*v1)
    return v2
}
"#,
    );

    program.assert_bytecode(
        r#"
function payload {
    address.add r1, r0, 8
    load.int64 r0, r1
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i64 8
    v3 = iadd v1, v2  ; v2 = 8
    v4 = load.i64 notrap aligned region0 v0+40
    v5 = iadd v4, v3
    v6 = load.i64 notrap aligned region1 v5
    return v6
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64 native
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

/// Address one constant element in frame and referenced storage.
#[test]
fn test_emit_constant_element_address() {
    let program = TestProgram::mir(
        r#"
export function third(v0: [int32; 3]): int32 {
entry(v0: [int32; 3]):
    v1: int32 = load (v0)[2]
    return v1
}

export function thirdReference<'a>(v0: ref<[int32; 3], borrowed, 'a, readonly>): int32 {
entry(v0: ref<[int32; 3], borrowed, 'a, readonly>):
    v1: int32 = load (*v0)[2]
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function third {
    frame.address r3, r0:r1
    address.add r3, r3, 8
    load.int32 r2, r3
    return r2
}

function thirdReference {
    address.add r2, r0, 8
    load.int32 r1, r2
    return r1
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
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i64 8
    v3 = iadd v1, v2  ; v2 = 8
    v4 = load.i32 notrap aligned region1 v3
    return v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 0
    v4 = iadd v1, v3  ; v3 = 0
    v5 = call fn0(v0, v4)
    store notrap aligned v5, v2
    return
}

function u0:2(i64 vmctx, i64) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i64 8
    v3 = iadd v1, v2  ; v2 = 8
    v4 = load.i64 notrap aligned region0 v0+40
    v5 = iadd v4, v3
    v6 = load.i32 notrap aligned region1 v5
    return v6
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i32 native
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

/// Follow a reference stored behind a reference, down to a slice and its length.
#[test]
fn test_emit_dereference_chain() {
    let program = TestProgram::mir(
        r#"
export function inner<'a, 'b>(v0: ref<ref<int32, borrowed, 'b, readonly>, borrowed, 'a, readonly>): int32 {
entry(v0: ref<ref<int32, borrowed, 'b, readonly>, borrowed, 'a, readonly>):
    v1: int32 = load (*(*v0))
    return v1
}

export function view<'a, 'b>(v0: ref<slice<int32, borrowed, 'b, readonly>, borrowed, 'a, readonly>): slice<int32, borrowed, 'b, readonly> {
entry(v0: ref<slice<int32, borrowed, 'b, readonly>, borrowed, 'a, readonly>):
    v1: slice<int32, borrowed, 'b, readonly> = address (*(*v0))
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function inner {
    load.uint64 r2, r0
    load.int32 r1, r2
    return r1
}

function view {
    address.add r3, r0, 8
    load.uint64 r3, r3
    load.uint64 r1, r0
    move r2, r3
    return r1:r2
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
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = load.i64 notrap aligned region0 v0+40
    v3 = iadd v2, v1
    v4 = load.i64 notrap aligned region1 v3
    v5 = load.i64 notrap aligned region0 v0+40
    v6 = iadd v5, v4
    v7 = load.i32 notrap aligned region1 v6
    return v7
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

function u0:2(i64 vmctx, i64) -> i64, i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = load.i64 notrap aligned region0 v0+40
    v3 = iadd v2, v1
    v4 = load.i64 notrap aligned region1 v3
    v5 = load.i64 notrap aligned region1 v3+8
    return v4, v5
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64, i64 native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4, v5 = call fn0(v0, v3)
    store notrap aligned v4, v2
    store notrap aligned v5, v2+8
    return
}
"#,
    );
}

/// Read a slice descriptor held in a local from its storage.
#[test]
fn test_emit_local_dereference() {
    let program = TestProgram::mir(
        r#"
export function select<'a>(v0: slice<int32, borrowed, 'a, readonly>, v1: usize): int32 {
    local l0: slice<int32, borrowed, 'a, readonly>

entry(v0: slice<int32, borrowed, 'a, readonly>, v1: usize):
    store l0, v0
    v2: slice<int32, borrowed, 'a, readonly> = address (*l0)
    v3: int32 = load (*l0)[v1]
    v4: int32 = load (*v2)[v1]
    v5: int32 = add v3, v4
    return v5
}
"#,
    );

    program.assert_bytecode(
        r#"
function select {
    move r5:r6, r0:r1
    move r0, r5
    move r1, r6
    address.add r7, r5, r2, 4
    load.int32 r3, r7
    address.add r7, r0, r2, 4
    load.int32 r4, r7
    add.int32 r0, r3, r4
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i64) -> i32 native {
    ss0 = explicit_slot 16, align = 8, key = 0
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i64):
    v4 = stack_addr.i64 ss0
    store notrap aligned region1 v1, v4
    store notrap aligned region1 v2, v4+8
    v5 = stack_addr.i64 ss0
    v6 = load.i64 notrap aligned region1 v5
    v7 = load.i64 notrap aligned region1 v5+8
    v8 = stack_addr.i64 ss0
    v9 = load.i64 notrap aligned region1 v8
    v10 = load.i64 notrap aligned region1 v8+8
    v11 = iconst.i64 4
    v12 = imul v3, v11  ; v11 = 4
    v13 = iadd v9, v12
    v14 = load.i64 notrap aligned region0 v0+40
    v15 = iadd v14, v13
    v16 = load.i32 notrap aligned region1 v15
    v17 = iconst.i64 4
    v18 = imul v3, v17  ; v17 = 4
    v19 = iadd v6, v18
    v20 = load.i64 notrap aligned region0 v0+40
    v21 = iadd v20, v19
    v22 = load.i32 notrap aligned region1 v21
    v23 = iadd v16, v22
    return v23
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i64) -> i32 native
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

/// Address a slice referent and a subslice of it.
#[test]
fn test_emit_slice_referent_address() {
    let program = TestProgram::mir(
        r#"
export function reborrow<'a>(v0: slice<int32, borrowed, 'a, readonly>, v1: usize, v2: usize): slice<int32, borrowed, 'a, readonly> {
entry(v0: slice<int32, borrowed, 'a, readonly>, v1: usize, v2: usize):
    v3: slice<int32, borrowed, 'a, readonly> = address (*v0)
    v4: slice<int32, borrowed, 'a, readonly> = address (*v3)[v1; v2]
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function reborrow {
    move r4, r0
    move r5, r1
    address.add r0, r4, r2, 4
    move r1, r3
    return r0:r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i64, i64) -> i64, i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
    v5 = iconst.i64 4
    v6 = imul v3, v5  ; v5 = 4
    v7 = iadd v1, v6
    return v7, v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i64, i64) -> i64, i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i64 notrap aligned v1+16
    v6 = load.i64 notrap aligned v1+24
    v7, v8 = call fn0(v0, v3, v4, v5, v6)
    store notrap aligned v7, v2
    store notrap aligned v8, v2+8
    return
}
"#,
    );
}

/// Access process memory through pointers without rebasing them on the world.
#[test]
fn test_emit_pointer_dereference() {
    let program = TestProgram::mir(
        r#"
export function transfer(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>, v2: usize, v3: uint8): int32 {
entry(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>, v2: usize, v3: uint8):
    v4: int32 = load (*v0)
    store (*v1), v4
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    intrinsic.memory.raw.setBytes(v0, v3, v2)
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function transfer {
    load.int32 r4, pointer r0
    store.int32 pointer r1, r4
    memory.copy pointer r0, pointer r1, r2
    memory.fill pointer r0, r3, r2
    return r4
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i64, i8) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    sig0 = (i64, i64, i64) -> i64 native
    sig1 = (i64, i32, i64) -> i64 native
    fn0 = %Memcpy sig0
    fn1 = %Memset sig1
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i8):
    v5 = load.i32 notrap aligned region1 v1
    store notrap aligned region1 v5, v2
    v6 = call fn0(v1, v2, v3)
    v7 = uextend.i32 v4
    v8 = call fn1(v1, v7, v3)
    return v5
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i64, i8) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i64 notrap aligned v1+16
    v6 = load.i8 notrap aligned v1+24
    v7 = call fn0(v0, v3, v4, v5, v6)
    store notrap aligned v7, v2
    return
}
"#,
    );
}

/// Reborrow dynamic and closure referents with their second descriptor word.
#[test]
fn test_emit_descriptor_referent_address() {
    let program = TestProgram::mir(
        r#"
type Writer { }

export function dynamic<'a>(v0: dynamic<Writer, borrowed, 'a, readonly>): dynamic<Writer, borrowed, 'a, readonly> {
entry(v0: dynamic<Writer, borrowed, 'a, readonly>):
    v1: dynamic<Writer, borrowed, 'a, readonly> = address (*v0)
    return v1
}

export function closure<'a>(v0: function<() => void, repeatable, borrowed, 'a, readonly>): function<() => void, repeatable, borrowed, 'a, readonly> {
entry(v0: function<() => void, repeatable, borrowed, 'a, readonly>):
    v1: function<() => void, repeatable, borrowed, 'a, readonly> = address (*v0)
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function dynamic {
    move r2, r0
    move r3, r1
    return r2:r3
}

function closure {
    move r3, r1
    move r2, r0
    return r2:r3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i32) -> i64, i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    return v1, v2
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) -> i64, i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5, v6 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    store notrap aligned v6, v2+8
    return
}

function u0:2(i64 vmctx, i64, i64) -> i64, i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64):
    return v1, v2
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64) -> i64, i64 native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5, v6 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    store notrap aligned v6, v2+8
    return
}
"#,
    );
}

/// Select a newtype's backing value at byte zero.
#[test]
fn test_emit_newtype_field() {
    let program = TestProgram::mir(
        r#"
type Meters = newtype<int64>;

export function meters<'a>(v0: ref<Meters, borrowed, 'a, readonly>): int64 {
entry(v0: ref<Meters, borrowed, 'a, readonly>):
    v1: int64 = load (*v0).0
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function meters {
    load.int64 r1, r0
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = load.i64 notrap aligned region0 v0+40
    v3 = iadd v2, v1
    v4 = load.i64 notrap aligned region1 v3
    return v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64 native
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
