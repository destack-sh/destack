use crate::tests::TestSession;

#[test]
fn test_lower_integer_addition_to_integer_add() {
    let session = TestSession::single(
        r#"
function add(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.add",
        r#"
function test.main.add(v0: int32, v1: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: int32 = load l1
    v4: int32 = add v2, v3
    return v4
}
"#,
    );
}

#[test]
fn test_lower_signed_arithmetic_operator_chain() {
    let session = TestSession::single(
        r#"
function calc(a: int32, b: int32): int32 {
    return a * b + a % b - b / a;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.calc",
        r#"
function test.main.calc(v0: int32, v1: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: int32 = load l1
    v4: int32 = mul v2, v3
    v5: int32 = load l0
    v6: int32 = load l1
    v7: int32 = rem v5, v6
    v8: int32 = add v4, v7
    v9: int32 = load l1
    v10: int32 = load l0
    v11: int32 = div v9, v10
    v12: int32 = sub v8, v11
    return v12
}
"#,
    );
}

#[test]
fn test_lower_unsigned_divide_and_remainder() {
    let session = TestSession::single(
        r#"
function split(x: uint32, d: uint32): uint32 {
    return x / d + x % d;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.split",
        r#"
function test.main.split(v0: uint32, v1: uint32): uint32 {
    local l0: uint32
    local l1: uint32

entry(v0: uint32, v1: uint32):
    store l0, v0
    store l1, v1
    v2: uint32 = load l0
    v3: uint32 = load l1
    v4: uint32 = div v2, v3
    v5: uint32 = load l0
    v6: uint32 = load l1
    v7: uint32 = rem v5, v6
    v8: uint32 = add v4, v7
    return v8
}
"#,
    );
}

#[test]
fn test_lower_unary_negate_to_integer_negate() {
    let session = TestSession::single(
        r#"
function flip(x: int32): int32 {
    return -x;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.flip",
        r#"
function test.main.flip(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = negate v1
    return v2
}
"#,
    );
}

#[test]
fn test_lower_folded_literal_operation_to_constant() {
    let session = TestSession::single(
        r#"
function three(): int32 {
    return 1 + 2;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.three",
        r#"
function test.main.three(): int32 {
entry:
    v0: int32 = 3
    return v0
}
"#,
    );
}
