use crate::tests::TestProgram;

/// Emit scalar and multiword function registers into canonical bytecode text.
#[test]
fn test_emit_function() {
    let program = TestProgram::mir(
        r#"
export function add(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    return v2
}

export function identity(v0: slice<int32, managed, mutable, local>): slice<int32, managed, mutable, local> {
entry(v0: slice<int32, managed, mutable, local>):
    return v0
}

@environment(ref<void, managed, mutable, local>)
export function closure(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function add {
    add.int32 r2, r0, r1
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

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32, v2: i32):
    v3 = iadd v1, v2
    return v3
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

function u0:4(i64 vmctx, i64, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    return v2
}

function u1:2(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) -> i32 native
    fn0 = colocated u0:4 sig0

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

/// Emit function pointers, captured environments, and environment projection.
#[test]
fn test_emit_function_values() {
    let program = TestProgram::mir(
        r#"
function increment(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = add v0, v1
    return v2
}

@environment(ref<void, managed, mutable, local>)
function captured(v0: int32): int32 {
entry(v0: int32):
    return v0
}

export function address(): fn(int32) => int32 {
entry:
    v0: fn(int32) => int32 = function.address increment
    return v0
}

@environment(ref<void, managed, mutable, local>)
export function environment(
    v0: ref<void, managed, mutable, local>,
): ref<void, managed, mutable, local> {
entry(v0: ref<void, managed, mutable, local>):
    v1: ref<void, managed, mutable, local> = function.environment.current
    v2: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind captured, v0
    v3: ref<void, managed, mutable, local> = function.environment v2
    return v3
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
    move r1, r4
    return r1
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

function u0:2(i64 vmctx, i64, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    return v2
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) -> i32 native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}

function u0:4(i64 vmctx) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    stack_limit = gv1

block0(v0: i64):
    v1 = symbol_value.i64 gv2
    v2 = load.i32 notrap aligned v1
    v3 = uextend.i64 v2
    v4 = iconst.i64 2
    v5 = iadd v3, v4  ; v4 = 2
    return v5
}

function u1:2(i64, i64, i64) native {
    sig0 = (i64 vmctx) -> i64 native
    fn0 = colocated u0:4 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = call fn0(v0)
    store notrap aligned v3, v2
    return
}

function u0:6(i64 vmctx, i64, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64):
    v3 = symbol_value.i64 gv2
    v4 = load.i32 notrap aligned v3
    v5 = uextend.i64 v4
    v6 = iconst.i64 2
    v7 = iadd v5, v6  ; v6 = 2
    return v2
}

function u1:3(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64) -> i64 native
    fn0 = colocated u0:6 sig0

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

/// Pass scalar parameters and results through the typed and runtime ABIs.
#[test]
fn test_emit_scalar_abi() {
    let program = TestProgram::mir(
        r#"
export function add(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    return v2
}
"#,
    );
    program.assert_bytecode(
        r#"
function add {
    add.int32 r2, r0, r1
    return r2
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
    v3 = iadd v1, v2
    return v3
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

/// Omit result parameters and stores for void functions.
#[test]
fn test_emit_void_abi() {
    let program = TestProgram::mir(
        r#"
export function discard(v0: int32): void {
entry(v0: int32):
    return
}
"#,
    );

    program.assert_bytecode(
        r#"
function discard {
    return
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32) native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32):
    return
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    call fn0(v0, v3)
    return
}
"#,
    );
}

/// Pass and return one two-scalar aggregate entirely in native registers.
#[test]
fn test_emit_scalar_pair_abi() {
    let program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int64;
}

export function identity(v0: Pair): Pair {
entry(v0: Pair):
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function identity {
    return r0:r1
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
"#,
    );
}

/// Pass and return larger aggregates through canonical addresses.
#[test]
fn test_emit_indirect_abi() {
    let program = TestProgram::mir(
        r#"
type Triple {
    first: int64;
    second: int64;
    third: int64;
}

export function identity(v0: Triple): Triple {
entry(v0: Triple):
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function identity {
    return r0:r2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64) native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned region1 v2
    store notrap aligned region1 v3, v1
    v4 = load.i64 notrap aligned region1 v2+8
    store notrap aligned region1 v4, v1+8
    v5 = load.i64 notrap aligned region1 v2+16
    store notrap aligned region1 v5, v1+16
    return
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 0
    v4 = iadd v1, v3  ; v3 = 0
    call fn0(v0, v2, v4)
    return
}
"#,
    );
}

/// Pass one hidden callable environment before explicit parameters.
#[test]
fn test_emit_environment_abi() {
    let program = TestProgram::mir(
        r#"
@environment(ref<void, managed, mutable, local>)
export function captured(v0: int32): ref<void, managed, mutable, local> {
entry(v0: int32):
    v1: ref<void, managed, mutable, local> = function.environment.current
    return v1
}
"#,
    );

    program.assert_bytecode(
        r#"
function captured {
    move r1, r0
    return r1
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
"#,
    );
}

/// Lower function addresses, closure construction, and environment projection.
#[test]
fn test_emit_function_binding() {
    let program = TestProgram::mir(
        r#"
function increment(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = add v0, v1
    return v2
}

@environment(ref<void, managed, mutable, local>)
function captured(v0: int32): int32 {
entry(v0: int32):
    return v0
}

export function address(): fn(int32) => int32 {
entry:
    v0: fn(int32) => int32 = function.address increment
    return v0
}

export function environment(
    v0: ref<void, managed, mutable, local>,
): ref<void, managed, mutable, local> {
entry(v0: ref<void, managed, mutable, local>):
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind captured, v0
    v2: ref<void, managed, mutable, local> = function.environment v1
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

function captured {
    return r1
}

function address {
    function.address r0, increment
    return r0
}

function environment {
    function.bind r1:r2, captured, r0
    move r0, r2
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

function u0:2(i64 vmctx, i64, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    return v2
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) -> i32 native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}

function u0:4(i64 vmctx) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    stack_limit = gv1

block0(v0: i64):
    v1 = symbol_value.i64 gv2
    v2 = load.i32 notrap aligned v1
    v3 = uextend.i64 v2
    v4 = iconst.i64 2
    v5 = iadd v3, v4  ; v4 = 2
    return v5
}

function u1:2(i64, i64, i64) native {
    sig0 = (i64 vmctx) -> i64 native
    fn0 = colocated u0:4 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = call fn0(v0)
    store notrap aligned v3, v2
    return
}

function u0:6(i64 vmctx, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = symbol_value.i64 gv2
    v3 = load.i32 notrap aligned v2
    v4 = uextend.i64 v3
    v5 = iconst.i64 2
    v6 = iadd v4, v5  ; v5 = 2
    return v1
}

function u1:3(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64 native
    fn0 = colocated u0:6 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}

/// Retain external declarations without inventing native definitions.
#[test]
fn test_emit_external_declaration() {
    let program = TestProgram::mir(
        r#"
external function consume(int32): int32
"#,
    );
    program.assert_bytecode(
        r#"
external function consume
"#,
    );

    let object = program.assert_native("");
    let [definition] = object.definitions() else {
        panic!("native object should retain one external declaration");
    };

    assert!(definition.get().is_none());
    assert!(object.blocks().is_empty());
}
