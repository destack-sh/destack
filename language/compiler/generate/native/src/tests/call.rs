//! Call instruction tests.
//!
//! Tests for lowering MIR call instructions to Cranelift IR.

use super::compile_mir_to_normalized_clif;

/// Simple direct call to a function with no arguments.
#[test]
fn test_direct_call_no_args() {
    let mir = r#"
function callee(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}

function caller(): int32 {
b0:
    v0: int32 = call callee()
    return v0
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // cranelift declares the signature separately with "sigN = ..."
    // and uses "fn0 = colocated u0:0 sig0" to reference it
    let expected = r#"
function u0:0(): int32 native {
b0:
    v0 = const.int32 42
    return v0
}

function u0:1(): int32 native {
    sig0 = () -> int32 native
    fn0 = colocated u0:0 sig0

b0:
    v0 = call fn0()
    return v0
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Direct call with arguments.
#[test]
fn test_direct_call_with_args() {
    let mir = r#"
function add(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function caller(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: int32 = call add(v0, v1)
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32, int32): int32 native {
b0(v0: int32, v1: int32):
    v2 = int.add v0, v1
    return v2
}

function u0:1(): int32 native {
    sig0 = (int32, int32) -> int32 native
    fn0 = colocated u0:0 sig0

b0:
    v0 = const.int32 10
    v1 = const.int32 20
    v2 = call fn0(v0, v1)
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Call to a void function (no return value).
#[test]
fn test_void_call() {
    let mir = r#"
function void_fn(): void {
b0:
    return
}

function caller(): void {
b0:
    call void_fn()
    return
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // void calls don't produce a value, so no "v0 =" prefix
    let expected = r#"
function u0:0() native {
b0:
    return
}

function u0:1() native {
    sig0 = () native
    fn0 = colocated u0:0 sig0

b0:
    call fn0()
    return
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Multiple calls to the same function.
#[test]
fn test_multiple_calls_same_function() {
    let mir = r#"
function double(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call double(v0)
    v2: int32 = call double(v1)
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32): int32 native {
b0(v0: int32):
    v1 = int.add v0, v0
    return v1
}

function u0:1(int32): int32 native {
    sig0 = (int32) -> int32 native
    fn0 = colocated u0:0 sig0

b0(v0: int32):
    v1 = call fn0(v0)
    v2 = call fn0(v1)
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Recursive call.
#[test]
fn test_recursive_call() {
    let mir = r#"
function factorial(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: boolean = int.le.s v0, v1
    branch v2, b1, b2

b1:
    return v1

b2:
    v3: int32 = int.sub v0, v1
    v4: int32 = call factorial(v3)
    v5: int32 = int.mul v0, v4
    return v5
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // recursive call: function calls itself
    let expected = r#"
function u0:0(int32): int32 native {
    sig0 = (int32) -> int32 native
    fn0 = colocated u0:0 sig0

b0(v0: int32):
    v1 = const.int32 1
    v2 = icmp sle v0, v1
    brif v2, b1, b2

b1:
    return v1

b2:
    v3 = int.sub.int32 v0, v1
    v4 = call fn0(v3)
    v5 = int.mul.int32 v0, v4
    return v5
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// callable.environment adds a hidden environment parameter to the signature.
#[test]
fn test_callable_environment_signature_param() {
    let mir = r#"
@environment(ref<int32, raw, space(stack)>)
function read_env(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = callable.environment
    v1: int32 = load v0
    return v1
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int64): int32 native {
b0(v0: int64):
    v1 = load.int32 v0
    return v1
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// call.indirect loads code and environment from a callable value.
#[test]
fn test_call_indirect_with_env_param() {
    let mir = r#"
@environment(ref<int32, raw, space(stack)>)
function read_env(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = callable.environment
    v1: int32 = load v0
    return v1
}

function caller(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    v2: () => int32 = callable.bind read_env, v0
    v3: int32 = call.indirect v2()
    return v3
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int64): int32 native {
b0(v0: int64):
    v1 = load.int32 v0
    return v1
}

function u0:1(): int32 native {
    ss0 = explicit_slot 4
    ss1 = explicit_slot 16, align = 8
    sig0 = (int64) -> int32 native
    sig1 = (int64) -> int32 native
    fn0 = colocated u0:0 sig0

b0:
    v0 = stack_addr.int64 ss0
    v1 = const.int32 7
    store v1, v0
    v2 = func_addr.int64 fn0
    v3 = stack_addr.int64 ss1
    store v2, v3
    store v0, v3+8
    v4 = load.int64 v3
    v5 = load.int64 v3+8
    v6 = call_indirect sig1, v4(v5)
    return v6
}"#
    .trim();
    assert_eq!(clif, expected);
}
