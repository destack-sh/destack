//! Call instruction tests.
//!
//! Tests for lowering MIR call instructions to Cranelift IR.

use super::compile_mir_to_normalized_clif;

/// Simple direct call to a function with no arguments.
#[test]
fn test_direct_call_no_args() {
    let mir = r#"
function @callee() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}

function @caller() -> i32 {
block0:
    v0 = call @callee()
    return v0
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // cranelift declares the signature separately with "sigN = ..."
    // and uses "fn0 = colocated u0:0 sig0" to reference it
    let expected = r#"
function u0:0() -> i32 native {
block0:
    v0 = iconst.i32 42
    return v0
}

function u0:1() -> i32 native {
    sig0 = () -> i32 native
    fn0 = colocated u0:0 sig0

block0:
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
function @add(v0: i32, v1: i32) -> i32 {
block0:
    v2 = iadd v0, v1
    return v2
}

function @caller() -> i32 {
block0:
    v0 = iconst 10i32
    v1 = iconst 20i32
    v2 = call @add(v0, v1)
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32, i32) -> i32 native {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}

function u0:1() -> i32 native {
    sig0 = (i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0:
    v0 = iconst.i32 10
    v1 = iconst.i32 20
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
function @void_fn() -> void {
block0:
    return
}

function @caller() -> void {
block0:
    call @void_fn()
    return
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // void calls don't produce a value, so no "v0 =" prefix
    let expected = r#"
function u0:0() native {
block0:
    return
}

function u0:1() native {
    sig0 = () native
    fn0 = colocated u0:0 sig0

block0:
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
function @double(v0: i32) -> i32 {
block0:
    v1 = iadd v0, v0
    return v1
}

function @caller(v0: i32) -> i32 {
block0:
    v1 = call @double(v0)
    v2 = call @double(v1)
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32) -> i32 native {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}

function u0:1(i32) -> i32 native {
    sig0 = (i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i32):
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
function @factorial(v0: i32) -> i32 {
block0:
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2

block1:
    return v1

block2:
    v3 = isub v0, v1
    v4 = call @factorial(v3)
    v5 = imul v0, v4
    return v5
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // recursive call: function calls itself
    let expected = r#"
function u0:0(i32) -> i32 native {
    sig0 = (i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    v1 = iconst.i32 1
    v2 = icmp sle v0, v1
    brif v2, block1, block2

block1:
    return v1

block2:
    v3 = isub.i32 v0, v1
    v4 = call fn0(v3)
    v5 = imul.i32 v0, v4
    return v5
}"#
    .trim();
    assert_eq!(clif, expected);
}
