use crate::tests::TestProgram;
use destack_bytecode as bytecode;

/// Pack scattered MIR values into one contiguous outgoing call window.
#[test]
fn test_emit_call_arguments() {
    let program = TestProgram::mir(
        r#"
external function consume(int32, boolean, int32): int32

export function caller(v0: int32, v1: boolean, v2: int32): int32 {
entry(v0: int32, v1: boolean, v2: int32):
    v3: int32 = add v0, v2
    v4: int32 = call consume(v3, v1, v0): (int32, boolean, int32) => int32
    return v4
}
"#,
    );

    let object = program.assert_bytecode(
        r#"
external function consume

function caller {
    add.int32 r3, r0, r2
    move r4, r3
    move r5, r1
    move r6, r0
    call r2, consume(r4:r6)
    return r2
}
"#,
    );

    // retain the semantic call coordinate after its outgoing register moves
    let bytecode = object
        .bytecode()
        .expect("bytecode emission should attach bytecode");
    let function = bytecode::FunctionId(1);
    let instruction = bytecode
        .operation(function, 1)
        .expect("caller operation should decode")
        .expect("caller operation should exist");
    assert_eq!(instruction.opcode(), bytecode::Opcode::CALL);

    program.assert_native(
        r#"
function u0:1(i64 vmctx, i32, i8, i32) -> i32 native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 4, align = 4, key = 4294967297
    ss2 = explicit_slot 1, key = 4294967298
    ss3 = explicit_slot 4, align = 4, key = 4294967299
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    sig0 = (i64 vmctx, i32, i8, i32) -> i32 native
    fn0 = colocated u0:0 sig0
    stack_limit = gv1

block0(v0: i64, v1: i32, v2: i8, v3: i32):
    v4 = iadd v1, v3
    v5 = stack_addr.i64 ss0
    v6 = stack_addr.i64 ss1
    store notrap aligned region1 v1, v6
    v7 = stack_addr.i64 ss2
    store notrap aligned region1 v2, v7
    v8 = stack_addr.i64 ss3
    store notrap aligned region1 v4, v8
    v9 = call fn0(v0, v4, v2, v1), stack_map=[i8 @ ss0+0, i8 @ ss1+0, i8 @ ss2+0, i8 @ ss3+0]
    return v9
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32, i8, i32) -> i32 native
    fn0 = colocated u0:1 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i8 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Route native calls through explicit normal and unwind continuations.
#[test]
fn test_emit_invoke() {
    let program = TestProgram::mir(
        r#"
external function callee(): int32

export function caller(): int32 {
entry:
    invoke callee(): () => int32 => returned | cleanup

returned(v0: int32):
    return v0

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_bytecode(
        r#"
external function callee

function caller {
    invoke r0, callee() => b0 | b1

b0:
    return r0

b1:
    unwind.resume
}
"#,
    );

    program.assert_native(
        r#"
function u0:1(i64 vmctx) -> i32 native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 8, align = 8
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    sig0 = (i64 vmctx) -> i32 native
    sig1 = (i64) -> i32 native
    sig2 = (i64, i64) native
    sig3 = (i64, i64) native
    fn0 = colocated u0:0 sig0
    stack_limit = gv1

block0(v1: i64):
    v2 = stack_addr.i64 ss0
    try_call fn0(v1), sig0, block3(ret0), [ default: block4(exn0, exn1) ], stack_map=[i8 @ ss0+0]

block3(v3: i32):
    jump block1(v3)

block4(v4: i64, v5: i64):
    v6 = stack_addr.i64 ss1
    store notrap aligned v4, v6
    v7 = load.i64 notrap aligned region0 v1+8
    v8 = load.i64 notrap aligned v7+80
    v9 = call_indirect sig1, v8(v1)
    brif v9, block5, block2

block5:
    v10 = load.i64 notrap aligned region0 v1+8
    v11 = load.i64 notrap aligned v10+88
    call_indirect sig2, v11(v1, v4)
    trap user4

block1(v0: i32):
    return v0

block2:
    v12 = stack_addr.i64 ss1
    v13 = load.i64 notrap aligned region1 v12
    v14 = load.i64 notrap aligned region0 v1+8
    v15 = load.i64 notrap aligned v14+88
    call_indirect sig3, v15(v1, v13)
    trap user4
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx) -> i32 native
    fn0 = colocated u0:1 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = call fn0(v0)
    store notrap aligned v3, v2
    return
}
"#,
    );
}

/// Preserve the caller frame required to resume after one native call.
#[test]
fn test_emit_caller_frame() {
    let program = TestProgram::mir(
        r#"
function double(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = add v0, v0
    return v1
}

export function advance(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call double(v0): (int32) => int32
    return v1
}
"#,
    );
    program.assert_bytecode(
        r#"
function double {
    add.int32 r1, r0, r0
    return r1
}

function advance {
    call r1, double(r0)
    return r1
}
"#,
    );

    let object = program.assert_native(
        r#"
function u0:0(i64 vmctx, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = iadd v1, v1
    return v2
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

function u0:2(i64 vmctx, i32) -> i32 native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 4, align = 4, key = 4294967297
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    sig0 = (i64 vmctx, i32) -> i32 native
    fn0 = colocated u0:0 sig0
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    v3 = stack_addr.i64 ss1
    store notrap aligned region1 v1, v3
    v4 = call fn0(v0, v1), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    return v4
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) -> i32 native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
    let sections = object.sections();
    let [frame] = object.map().frames(sections) else {
        panic!("native caller should emit one frame map");
    };
    let caller = object.definitions()[1]
        .get()
        .expect("native caller should retain one definition");

    // associate the call state with the caller's typed body
    assert_eq!(frame.block, caller.body);
}

/// Invoke one bare function pointer through the internal native ABI.
#[test]
fn test_emit_function_pointer_call() {
    let program = TestProgram::mir(
        r#"
function increment(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = add v0, v1
    return v2
}

export function dispatch(v0: int32): int32 {
entry(v0: int32):
    v1: fn(int32) => int32 = function.address increment
    v2: int32 = call.indirect v1(v0): (int32) => int32
    return v2
}
"#,
    );

    program.assert_bytecode(
        r#"
function increment {
    constant.int32 r1, 1
    add.int32 r2, r0, r1
    return r2
}

function dispatch {
    function.address r1, increment
    call.indirect r2, r1(r0)
    return r2
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
    v2 = iconst.i32 1
    v3 = iadd v1, v2  ; v2 = 1
    return v3
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

function u0:2(i64 vmctx, i32) -> i32 native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 4, align = 4, key = 4294967297
    ss2 = explicit_slot 8, align = 8, key = 4294967298
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    gv3 = symbol colocated userextname1
    sig0 = (i64, i32, i64) native
    sig1 = (i64 vmctx, i32) -> i32 native
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = symbol_value.i64 gv2
    v3 = load.i32 notrap aligned v2
    v4 = uextend.i64 v3
    v5 = iconst.i64 2
    v6 = iadd v4, v5  ; v5 = 2
    v7 = stack_addr.i64 ss0
    v8 = stack_addr.i64 ss1
    store notrap aligned region1 v1, v8
    v9 = stack_addr.i64 ss2
    store notrap aligned region1 v6, v9
    v10 = load.i64 notrap aligned region0 v0+16
    v11 = iconst.i64 8
    v12 = imul v6, v11  ; v11 = 8
    v13 = iadd v10, v12
    v14 = load.i64 notrap aligned v13
    v15 = iconst.i64 0
    v16 = icmp ne v14, v15  ; v15 = 0
    brif v16, block1, block2

block2:
    v17 = symbol_value.i64 gv3
    v18 = load.i32 notrap aligned v17
    v19 = load.i64 notrap aligned region0 v0+8
    v20 = load.i64 notrap aligned v19+56
    call_indirect sig0, v20(v0, v18, v7), stack_map=[i8 @ ss0+0, i8 @ ss1+0, i8 @ ss2+0]
    trap user4

block1:
    v21 = call_indirect.i64 sig1, v14(v0, v1), stack_map=[i8 @ ss0+0, i8 @ ss1+0, i8 @ ss2+0]
    return v21
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) -> i32 native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}

/// Invoke one closure with its captured environment before explicit arguments.
#[test]
fn test_emit_closure_call() {
    let program = TestProgram::mir(
        r#"
@environment(ref<void, managed, readonly, local>)
function captured(v0: int32): ref<void, managed, readonly, local> {
entry(v0: int32):
    v1: ref<void, managed, readonly, local> = function.environment.current
    return v1
}

export function dispatch(
    v0: ref<void, managed, readonly, local>,
    v1: int32,
): ref<void, managed, readonly, local> {
entry(v0: ref<void, managed, readonly, local>, v1: int32):
    v2: function<(int32) => ref<void, managed, readonly, local>, repeatable, managed, readonly, local> = function.bind captured, v0
    v3: ref<void, managed, readonly, local> = call.indirect v2(v1): (int32) => ref<void, managed, readonly, local>
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function captured {
    move r1, r0
    return r1
}

function dispatch {
    function.bind r2:r3, captured, r0
    call.indirect r0, r2:r3(r1)
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i32) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    return v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}

function u0:2(i64 vmctx, i64, i32) -> i64 native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 4, align = 4, key = 4294967297
    ss2 = explicit_slot 16, align = 8, key = 4294967298
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    gv3 = symbol colocated userextname1
    sig0 = (i64, i32, i64) native
    sig1 = (i64 vmctx, i64, i32) -> i64 native
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    v3 = symbol_value.i64 gv2
    v4 = load.i32 notrap aligned v3
    v5 = uextend.i64 v4
    v6 = iconst.i64 2
    v7 = iadd v5, v6  ; v6 = 2
    v8 = stack_addr.i64 ss0
    v9 = stack_addr.i64 ss1
    store notrap aligned region1 v2, v9
    v10 = stack_addr.i64 ss2
    store notrap aligned region1 v7, v10
    store notrap aligned region1 v1, v10+8
    v11 = load.i64 notrap aligned region0 v0+16
    v12 = iconst.i64 8
    v13 = imul v7, v12  ; v12 = 8
    v14 = iadd v11, v13
    v15 = load.i64 notrap aligned v14
    v16 = iconst.i64 0
    v17 = icmp ne v15, v16  ; v16 = 0
    brif v17, block1, block2

block2:
    v18 = symbol_value.i64 gv3
    v19 = load.i32 notrap aligned v18
    v20 = load.i64 notrap aligned region0 v0+8
    v21 = load.i64 notrap aligned v20+56
    call_indirect sig0, v21(v0, v19, v8), stack_map=[i8 @ ss0+0, i8 @ ss1+0, i8 @ ss2+0]
    trap user4

block1:
    v22 = call_indirect.i64 sig1, v15(v0, v1, v2), stack_map=[i8 @ ss0+0, i8 @ ss1+0, i8 @ ss2+0]
    return v22
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) -> i64 native
    fn0 = colocated u0:2 sig0

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

/// Marshal one imported binding through the runtime Word ABI.
#[test]
fn test_emit_binding_call() {
    let program = TestProgram::mir(
        r#"
@binding("runtime.touch", { provider: "runtime", effect: "deterministic", replay: "forbidden", affinity: "worker" })
external function touch(int32): int32

export function dispatch(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call touch(v0): (int32) => int32
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
external function touch

function dispatch {
    call r1, touch(r0)
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:1(i64 vmctx, i32) -> i32 native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 4, align = 4, key = 4294967297
    ss2 = explicit_slot 8, align = 8
    ss3 = explicit_slot 8, align = 8
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    sig0 = (i64, i32, i64, i64, i64, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    v3 = stack_addr.i64 ss1
    store notrap aligned region1 v1, v3
    v4 = stack_addr.i64 ss2
    v5 = iconst.i64 0
    v6 = iadd v4, v5  ; v5 = 0
    v7 = sextend.i64 v1
    store notrap aligned region1 v7, v6
    v8 = stack_addr.i64 ss3
    v9 = iconst.i64 0
    store notrap aligned region1 v9, v8  ; v9 = 0
    v10 = symbol_value.i64 gv2
    v11 = load.i32 notrap aligned v10
    v12 = iconst.i64 1
    v13 = iconst.i64 1
    v14 = load.i64 notrap aligned region0 v0+8
    v15 = load.i64 notrap aligned v14+120
    call_indirect sig0, v15(v0, v11, v4, v12, v8, v13), stack_map=[i8 @ ss0+0, i8 @ ss1+0]  ; v12 = 1, v13 = 1
    v16 = load.i32 notrap aligned region1 v8
    return v16
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) -> i32 native
    fn0 = colocated u0:1 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}
