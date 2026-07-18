use crate::tests::TestSession;

#[test]
fn test_lower_signed_ordering_to_signed_compare() {
    let session = TestSession::single(
        r#"
function less(a: int32, b: int32): boolean {
    return a < b;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.less(v0: int32, v1: int32): boolean {
entry(v0: int32, v1: int32):
    v2: boolean = int.lt.s v0, v1
    return v2
}
"#,
    );
}

#[test]
fn test_lower_unsigned_ordering_to_unsigned_compare() {
    let session = TestSession::single(
        r#"
function above(a: uint32, b: uint32): boolean {
    return a > b;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.above(v0: uint32, v1: uint32): boolean {
entry(v0: uint32, v1: uint32):
    v2: boolean = int.gt.u v0, v1
    return v2
}
"#,
    );
}

#[test]
fn test_lower_integer_equality_to_integer_compare() {
    let session = TestSession::single(
        r#"
function same(a: int64, b: int64): boolean {
    return a == b;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.same(v0: int64, v1: int64): boolean {
entry(v0: int64, v1: int64):
    v2: boolean = int.eq v0, v1
    return v2
}
"#,
    );
}

#[test]
fn test_lower_literal_comparison_over_family_carrier() {
    let session = TestSession::single(
        r#"
function yes(): boolean {
    return 1 < 2;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.yes(): boolean {
entry:
    v0: float64 = 1
    v1: float64 = 2
    v2: boolean = float.lt v0, v1
    return v2
}
"#,
    );
}
