use crate::tests::TestProgram;
use tspp_native::BlockId;
use tspp_native::abi::Trap;

/// Transport block arguments through conditional edges.
#[test]
fn test_emit_block_arguments() {
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

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i8, i32, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v1: i64, v2: i8, v3: i32, v4: i32):
    brif v2, block1(v3), block1(v4)

block1(v0: i32):
    return v0
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i8, i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i8 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Emit compact switch dispatch and edge argument transfers.
#[test]
fn test_emit_switch() {
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

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32, i32, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v2: i64, v3: i32, v4: i32, v5: i32):
    v6 = iconst.i32 17
    v7 = icmp eq v3, v6  ; v6 = 17
    brif v7, block5, block6

block6:
    brif.i32 v3, block3, block4

block3:
    jump block2(v5)

block4:
    jump block1(v4)

block5:
    jump block1(v5)

block1(v0: i32):
    return v0

block2(v1: i32):
    return v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32, i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Emit explicit scalar checks with normal and failure continuations.
#[test]
fn test_emit_check_branches() {
    let program = TestProgram::mir(
        r#"
export function guard(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    check bounds.s v0, v1, v0 => valid | invalid

valid:
    check add.overflow.s v0, v1 => result | invalid

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
    check.bounds.int32 r0, r1 | b2
    jump b0

b0:
    check.add.overflow.int32 r0, r1 | b2
    jump b1

b1:
    return r0

b2:
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32, v2: i32):
    v3 = icmp slt v1, v2
    v4 = iconst.i32 0
    v5 = icmp sge v1, v4  ; v4 = 0
    v6 = band v5, v3
    brif v6, block1, block3

block1:
    v7, v8 = sadd_overflow.i32 v1, v2
    v9 = iconst.i8 0
    v10 = icmp eq v8, v9  ; v9 = 0
    brif v10, block2, block3

block2:
    return v1

block3:
    return v2
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Emit unreachable control flow as one terminal bytecode operation.
#[test]
fn test_emit_unreachable() {
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

    let object = program.assert_native(
        r#"
function u0:0(i64 vmctx) native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64):
    trap user4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    call fn0(v0)
    return
}
"#,
    );

    // retain the explicit MIR trap in language terms
    let traps = object
        .map()
        .traps(object.sections())
        .iter()
        .map(|trap| (trap.block, trap.trap))
        .collect::<Vec<_>>();
    assert_eq!(traps, [(BlockId(0), Trap::Unreachable)]);
}

/// Transport block arguments through conditional and switch edges.
#[test]
fn test_emit_branch_and_switch_arguments() {
    let program = TestProgram::mir(
        r#"
export function dispatch(
    v0: int32,
    v1: boolean,
    v2: int32,
    v3: int32,
): int32 {
entry(v0: int32, v1: boolean, v2: int32, v3: int32):
    branch v1 => b1(v2) | b1(v3)

b1(v4: int32):
    switch v0, b4(v4), 0 => b2(v2), 17 => b3(v3)

b2(v5: int32):
    return v5

b3(v6: int32):
    return v6

b4(v7: int32):
    return v7
}
"#,
    );

    program.assert_bytecode(
        r#"
function dispatch {
    branch r1 => b4 | b5

b0:
    switch r0 { 0 => b6, 17 => b7, default => b8 }

b1:
    return r0

b2:
    return r0

b3:
    return r0

b4:
    move r1, r2
    jump b0

b5:
    move r1, r3
    jump b0

b6:
    move r0, r2
    jump b1

b7:
    move r0, r3
    jump b2

b8:
    move r0, r1
    jump b3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32, i8, i32, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v4: i64, v5: i32, v6: i8, v7: i32, v8: i32):
    brif v6, block1(v7), block1(v8)

block1(v0: i32):
    v9 = iconst.i32 17
    v10 = icmp.i32 eq v5, v9  ; v9 = 17
    brif v10, block7, block8

block8:
    brif.i32 v5, block5, block6

block5:
    jump block4(v0)

block6:
    jump block2(v7)

block7:
    jump block3(v8)

block2(v1: i32):
    return v1

block3(v2: i32):
    return v2

block4(v3: i32):
    return v3
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32, i8, i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i8 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = load.i32 notrap aligned v1+24
    v7 = call fn0(v0, v3, v4, v5, v6)
    store notrap aligned v7, v2
    return
}
"#,
    );
}

/// Emit one dense integer switch as a native branch table.
#[test]
fn test_emit_dense_switch() {
    let program = TestProgram::mir(
        r#"
export function select(v0: int32): int32 {
entry(v0: int32):
    switch v0, b4, 0 => b1, 1 => b2, 2 => b3

b1:
    v1: int32 = 10
    return v1

b2:
    v2: int32 = 20
    return v2

b3:
    v3: int32 = 30
    return v3

b4:
    v4: int32 = 40
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function select {
    switch r0 { 0 => b0, 1 => b1, 2 => b2, default => b3 }

b0:
    constant.int32 r0, 10
    return r0

b1:
    constant.int32 r0, 20
    return r0

b2:
    constant.int32 r0, 30
    return r0

b3:
    constant.int32 r0, 40
    return r0
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
    stack_limit = gv1

block0(v0: i64, v1: i32):
    br_table v1, block4, [block1, block2, block3]

block1:
    v2 = iconst.i32 10
    return v2  ; v2 = 10

block2:
    v3 = iconst.i32 20
    return v3  ; v3 = 20

block3:
    v4 = iconst.i32 30
    return v4  ; v4 = 30

block4:
    v5 = iconst.i32 40
    return v5  ; v5 = 40
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

/// Lower bounds, null, division, shift, and narrowing guards.
#[test]
fn test_emit_scalar_checks() {
    let program = TestProgram::mir(
        r#"
export function guard(
    v0: int32,
    v1: int32,
    v2: uint32,
    v3: uint32,
    v4: ptr<int32, mutable>,
    v5: int64,
    v6: uint64,
): int32 {
entry(v0: int32, v1: int32, v2: uint32, v3: uint32, v4: ptr<int32, mutable>, v5: int64, v6: uint64):
    check bounds.s v0, v1, v4 => b1 | b10

b1:
    check bounds.u v2, v3, v4 => b2 | b10

b2:
    check null v4 => b3 | b10

b3:
    check div.zero v0 => b4 | b10

b4:
    check shift.range.s v0, 32 => b5 | b10

b5:
    check shift.range.u v2, 32 => b6 | b10

b6:
    check narrow.range.s v5, 8 => b7 | b10

b7:
    check narrow.range.u v5, 8 => b8 | b10

b8:
    check narrow.range.s v6, 8 => b9 | b10

b9:
    return v0

b10:
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function guard {
    check.bounds.int32 r0, r1 | b9
    jump b0

b0:
    check.bounds.uint32 r2, r3 | b9
    jump b1

b1:
    check.nullish r4 | b9
    jump b2

b2:
    check.nonzero.int32 r0 | b9
    jump b3

b3:
    check.shift.int32 r0, 32 | b9
    jump b4

b4:
    check.shift.uint32 r2, 32 | b9
    jump b5

b5:
    check.narrow.int64.int8 r5 | b9
    jump b6

b6:
    check.narrow.int64.uint8 r5 | b9
    jump b7

b7:
    check.narrow.uint64.int8 r6 | b9
    jump b8

b8:
    return r0

b9:
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32, i32, i32, i32, i64, i64, i64) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32, v2: i32, v3: i32, v4: i32, v5: i64, v6: i64, v7: i64):
    v8 = icmp slt v1, v2
    v9 = iconst.i32 0
    v10 = icmp sge v1, v9  ; v9 = 0
    v11 = band v10, v8
    brif v11, block1, block10

block1:
    v12 = icmp.i32 ult v3, v4
    brif v12, block2, block10

block2:
    v13 = iconst.i64 0
    v14 = icmp.i64 ne v5, v13  ; v13 = 0
    brif v14, block3, block10

block3:
    v15 = iconst.i32 0
    v16 = icmp.i32 ne v1, v15  ; v15 = 0
    brif v16, block4, block10

block4:
    v17 = iconst.i32 32
    v18 = icmp.i32 slt v1, v17  ; v17 = 32
    v19 = iconst.i32 0
    v20 = icmp.i32 sge v1, v19  ; v19 = 0
    v21 = band v20, v18
    brif v21, block5, block10

block5:
    v22 = iconst.i32 32
    v23 = icmp.i32 ult v3, v22  ; v22 = 32
    brif v23, block6, block10

block6:
    v24 = iconst.i64 -128
    v25 = icmp.i64 sge v6, v24  ; v24 = -128
    v26 = iconst.i64 127
    v27 = icmp.i64 sle v6, v26  ; v26 = 127
    v28 = band v25, v27
    brif v28, block7, block10

block7:
    v29 = iconst.i64 0
    v30 = icmp.i64 sge v6, v29  ; v29 = 0
    v31 = iconst.i64 255
    v32 = icmp.i64 sle v6, v31  ; v31 = 255
    v33 = band v30, v32
    brif v33, block8, block10

block8:
    v34 = iconst.i64 127
    v35 = icmp.i64 ule v7, v34  ; v34 = 127
    brif v35, block9, block10

block9:
    return v1

block10:
    return v2
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32, i32, i32, i32, i64, i64, i64) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = load.i32 notrap aligned v1+24
    v7 = load.i64 notrap aligned v1+32
    v8 = load.i64 notrap aligned v1+40
    v9 = load.i64 notrap aligned v1+48
    v10 = call fn0(v0, v3, v4, v5, v6, v7, v8, v9)
    store notrap aligned v10, v2
    return
}
"#,
    );
}

/// Lower exact and transitive Program type guards.
#[test]
fn test_emit_type_checks() {
    let program = TestProgram::mir(
        r#"
type Expected { }

export function guardType(v0: typeId): int32 {
entry(v0: typeId):
    check is.type v0, Expected => b0 | b2

b0:
    check is.subtype v0, Expected => b1 | b2

b1:
    v1: int32 = 1
    return v1

b2:
    v2: int32 = 0
    return v2
}
"#,
    );

    program.assert_bytecode(
        r#"
function guardType {
    check.type r0, t0 | b2
    jump b0

b0:
    check.subtype r0, t0 | b2
    jump b1

b1:
    constant.int32 r0, 1
    return r0

b2:
    constant.int32 r0, 0
    return r0
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
    sig0 = (i64, i32, i32) -> i32 native
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = symbol_value.i64 gv2
    v3 = load.i32 notrap aligned v2
    v4 = icmp eq v1, v3
    brif v4, block1, block3

block1:
    v5 = symbol_value.i64 gv2
    v6 = load.i32 notrap aligned v5
    v7 = load.i64 notrap aligned region0 v0+8
    v8 = load.i64 notrap aligned v7+96
    v9 = call_indirect sig0, v8(v0, v1, v6)
    v10 = iconst.i32 0
    v11 = icmp ne v9, v10  ; v10 = 0
    brif v11, block2, block3

block2:
    v12 = iconst.i32 1
    return v12  ; v12 = 1

block3:
    v13 = iconst.i32 0
    return v13  ; v13 = 0
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

/// Lower signed and unsigned arithmetic overflow guards.
#[test]
fn test_emit_overflow_checks() {
    let program = TestProgram::mir(
        r#"
export function guardOverflow(
    v0: int32,
    v1: int32,
    v2: uint32,
    v3: uint32,
): int32 {
entry(v0: int32, v1: int32, v2: uint32, v3: uint32):
    check add.overflow.s v0, v1 => b1 | b7

b1:
    check sub.overflow.s v0, v1 => b2 | b7

b2:
    check mul.overflow.s v0, v1 => b3 | b7

b3:
    check add.overflow.u v2, v3 => b4 | b7

b4:
    check sub.overflow.u v2, v3 => b5 | b7

b5:
    check mul.overflow.u v2, v3 => b6 | b7

b6:
    return v0

b7:
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function guardOverflow {
    check.add.overflow.int32 r0, r1 | b6
    jump b0

b0:
    check.sub.overflow.int32 r0, r1 | b6
    jump b1

b1:
    check.mul.overflow.int32 r0, r1 | b6
    jump b2

b2:
    check.add.overflow.uint32 r2, r3 | b6
    jump b3

b3:
    check.sub.overflow.uint32 r2, r3 | b6
    jump b4

b4:
    check.mul.overflow.uint32 r2, r3 | b6
    jump b5

b5:
    return r0

b6:
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32, i32, i32, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32, v2: i32, v3: i32, v4: i32):
    v5, v6 = sadd_overflow v1, v2
    v7 = iconst.i8 0
    v8 = icmp eq v6, v7  ; v7 = 0
    brif v8, block1, block7

block1:
    v9, v10 = ssub_overflow.i32 v1, v2
    v11 = iconst.i8 0
    v12 = icmp eq v10, v11  ; v11 = 0
    brif v12, block2, block7

block2:
    v13, v14 = smul_overflow.i32 v1, v2
    v15 = iconst.i8 0
    v16 = icmp eq v14, v15  ; v15 = 0
    brif v16, block3, block7

block3:
    v17, v18 = uadd_overflow.i32 v3, v4
    v19 = iconst.i8 0
    v20 = icmp eq v18, v19  ; v19 = 0
    brif v20, block4, block7

block4:
    v21, v22 = usub_overflow.i32 v3, v4
    v23 = iconst.i8 0
    v24 = icmp eq v22, v23  ; v23 = 0
    brif v24, block5, block7

block5:
    v25, v26 = umul_overflow.i32 v3, v4
    v27 = iconst.i8 0
    v28 = icmp eq v26, v27  ; v27 = 0
    brif v28, block6, block7

block6:
    return v1

block7:
    return v2
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = load.i32 notrap aligned v1+24
    v7 = call fn0(v0, v3, v4, v5, v6)
    store notrap aligned v7, v2
    return
}
"#,
    );
}
