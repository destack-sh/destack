use crate::tests::TestProgram;

/// Emit execution context operations with one concrete node allocation site.
#[test]
fn test_emit_context() {
    let program = TestProgram::mir(
        r#"
type Context { }

type Variable { }

type ContextNode {
    parent: ref<Context, managed, readonly, local>;
    variable: ref<Variable, managed, readonly, local>;
    value: int32;
}

export function scope(v0: ref<Variable, managed, readonly, local>, v1: int32): int32 {
entry(v0: ref<Variable, managed, readonly, local>, v1: int32):
    v2: ref<Context, managed, readonly, local> = context.current
    v3: ref<Context, managed, readonly, local> = context.bind v2, v0, v1, ContextNode
    v4: ref<Context, managed, readonly, local> = context.replace v3
    v5: int32 = context.get v3, v0, v1, ContextNode
    v6: ref<Context, managed, readonly, local> = context.replace v4
    return v5
}
"#,
    );

    program.assert_bytecode(
        r#"
function scope {
    context.current r2
    context.bind r3, r2, r0, r1, a0, 16
    context.replace r2, r3
    context.get r4, r3, r0, r1, 16
    context.replace r0, r2
    return r4
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
    gv2 = symbol colocated userextname0
    sig0 = (i64, i32, i32, i32) -> i64 native
    sig1 = (i64, i64, i64, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    v3 = load.i64 notrap aligned region0 v0+104
    v4 = iconst.i32 0
    v5 = symbol_value.i64 gv2
    v6 = load.i32 notrap aligned v5
    v7 = iconst.i32 0
    v8 = load.i64 notrap aligned region0 v0+8
    v9 = load.i64 notrap aligned v8
    v10 = call_indirect sig0, v9(v0, v4, v6, v7)  ; v4 = 0, v7 = 0
    v11 = load.i64 notrap aligned region0 v0+40
    v12 = iadd v11, v10
    store notrap aligned region1 v3, v12
    store notrap aligned region1 v1, v12+8
    v13 = iconst.i64 16
    v14 = iadd v12, v13  ; v13 = 16
    store notrap aligned region1 v2, v14
    v15 = iconst.i64 0
    v16 = iconst.i64 24
    v17 = load.i64 notrap aligned region0 v0+8
    v18 = load.i64 notrap aligned v17+32
    call_indirect sig1, v18(v0, v12, v15, v16)  ; v15 = 0, v16 = 24
    v19 = load.i64 notrap aligned region0 v0+104
    store notrap aligned region0 v10, v0+104
    v20 = load.i64 notrap aligned region0 v0+40
    jump block1(v10)

block1(v21: i64):
    v23 = iconst.i64 0
    v24 = icmp eq v21, v23  ; v23 = 0
    brif v24, block3, block2

block2:
    v25 = iadd.i64 v20, v21
    v26 = load.i64 notrap aligned region1 v25+8
    v27 = icmp eq v26, v1
    brif v27, block4, block5

block4:
    v28 = iconst.i64 16
    v29 = iadd.i64 v25, v28  ; v28 = 16
    v30 = load.i32 notrap aligned region1 v29
    jump block6(v30)

block5:
    v31 = load.i64 notrap aligned region1 v25
    jump block1(v31)

block3:
    jump block6(v2)

block6(v22: i32):
    v32 = load.i64 notrap aligned region0 v0+104
    store.i64 notrap aligned region0 v19, v0+104
    return v22
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

/// Load the execution context owned by the activation.
#[test]
fn test_emit_context_current() {
    let program = TestProgram::mir(
        r#"
type Context { }

export function current(): ref<Context, managed, readonly, local> {
entry:
    v0: ref<Context, managed, readonly, local> = context.current
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function current {
    context.current r0
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64):
    v1 = load.i64 notrap aligned region0 v0+104
    return v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = call fn0(v0)
    store notrap aligned v3, v2
    return
}
"#,
    );
}

/// Replace the execution context owned by the activation.
#[test]
fn test_emit_context_replace() {
    let program = TestProgram::mir(
        r#"
type Context { }

export function replace(
    v0: ref<Context, managed, readonly, local>,
): ref<Context, managed, readonly, local> {
entry(v0: ref<Context, managed, readonly, local>):
    v1: ref<Context, managed, readonly, local> = context.replace v0
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function replace {
    context.replace r1, r0
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
    v2 = load.i64 notrap aligned region0 v0+104
    store notrap aligned region0 v1, v0+104
    return v2
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

/// Allocate and initialize one immutable execution context node.
#[test]
fn test_emit_context_bind() {
    let program = TestProgram::mir(
        r#"
type Context { }

type Variable { }

type ContextNode {
    parent: ref<Context, managed, readonly, local>;
    variable: ref<Variable, managed, readonly, local>;
    value: int32;
}

export function bind(
    v0: ref<Context, managed, readonly, local>,
    v1: ref<Variable, managed, readonly, local>,
    v2: int32,
): ref<Context, managed, readonly, local> {
entry(v0: ref<Context, managed, readonly, local>, v1: ref<Variable, managed, readonly, local>, v2: int32):
    v3: ref<Context, managed, readonly, local> = context.bind v0, v1, v2, ContextNode
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function bind {
    context.bind r3, r0, r1, r2, a0, 16
    return r3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i32) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    sig0 = (i64, i32, i32, i32) -> i64 native
    sig1 = (i64, i64, i64, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i32):
    v4 = iconst.i32 0
    v5 = symbol_value.i64 gv2
    v6 = load.i32 notrap aligned v5
    v7 = iconst.i32 0
    v8 = load.i64 notrap aligned region0 v0+8
    v9 = load.i64 notrap aligned v8
    v10 = call_indirect sig0, v9(v0, v4, v6, v7)  ; v4 = 0, v7 = 0
    v11 = load.i64 notrap aligned region0 v0+40
    v12 = iadd v11, v10
    store notrap aligned region1 v1, v12
    store notrap aligned region1 v2, v12+8
    v13 = iconst.i64 16
    v14 = iadd v12, v13  ; v13 = 16
    store notrap aligned region1 v3, v14
    v15 = iconst.i64 0
    v16 = iconst.i64 24
    v17 = load.i64 notrap aligned region0 v0+8
    v18 = load.i64 notrap aligned v17+32
    call_indirect sig1, v18(v0, v12, v15, v16)  ; v15 = 0, v16 = 24
    return v10
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i32) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Search an immutable execution context chain with an explicit default.
#[test]
fn test_emit_context_get() {
    let program = TestProgram::mir(
        r#"
type Context { }

type Variable { }

type ContextNode {
    parent: ref<Context, managed, readonly, local>;
    variable: ref<Variable, managed, readonly, local>;
    value: int32;
}

export function get(
    v0: ref<Context, managed, readonly, local>,
    v1: ref<Variable, managed, readonly, local>,
    v2: int32,
): int32 {
entry(v0: ref<Context, managed, readonly, local>, v1: ref<Variable, managed, readonly, local>, v2: int32):
    v3: int32 = context.get v0, v1, v2, ContextNode
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function get {
    context.get r3, r0, r1, r2, 16
    return r3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i32):
    v4 = load.i64 notrap aligned region0 v0+40
    jump block1(v1)

block1(v5: i64):
    v7 = iconst.i64 0
    v8 = icmp eq v5, v7  ; v7 = 0
    brif v8, block3, block2

block2:
    v9 = iadd.i64 v4, v5
    v10 = load.i64 notrap aligned region1 v9+8
    v11 = icmp eq v10, v2
    brif v11, block4, block5

block4:
    v12 = iconst.i64 16
    v13 = iadd.i64 v9, v12  ; v12 = 16
    v14 = load.i32 notrap aligned region1 v13
    jump block6(v14)

block5:
    v15 = load.i64 notrap aligned region1 v9
    jump block1(v15)

block3:
    jump block6(v3)

block6(v6: i32):
    return v6
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}
