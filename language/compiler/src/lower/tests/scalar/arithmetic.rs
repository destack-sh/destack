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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function add(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function calc(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.mul v0, v1
    v3: int32 = int.rem.s v0, v1
    v4: int32 = int.add v2, v3
    v5: int32 = int.div.s v1, v0
    v6: int32 = int.sub v4, v5
    return v6
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function split(v0: uint32, v1: uint32): uint32 {
entry(v0: uint32, v1: uint32):
    v2: uint32 = int.div.u v0, v1
    v3: uint32 = int.rem.u v0, v1
    v4: uint32 = int.add v2, v3
    return v4
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function flip(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.negate v0
    return v1
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function three(): int32 {
entry:
    v0: int32 = 3
    return v0
}
"#,
    );
}
