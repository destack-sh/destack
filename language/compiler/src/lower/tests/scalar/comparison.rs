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

    session.assert_mir_function(
        "main.ds",
        "test.main.less",
        r#"
function test.main.less(v0: int32, v1: int32): boolean {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: int32 = load l1
    v4: boolean = lt v2, v3
    return v4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.above",
        r#"
function test.main.above(v0: uint32, v1: uint32): boolean {
    local l0: uint32
    local l1: uint32

entry(v0: uint32, v1: uint32):
    store l0, v0
    store l1, v1
    v2: uint32 = load l0
    v3: uint32 = load l1
    v4: boolean = gt v2, v3
    return v4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.same",
        r#"
function test.main.same(v0: int64, v1: int64): boolean {
    local l0: int64
    local l1: int64

entry(v0: int64, v1: int64):
    store l0, v0
    store l1, v1
    v2: int64 = load l0
    v3: int64 = load l1
    v4: boolean = eq v2, v3
    return v4
}
"#,
    );
}

#[test]
fn test_lower_literal_comparison_over_family_representation() {
    let session = TestSession::single(
        r#"
function yes(): boolean {
    return 1 < 2;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.yes",
        r#"
function test.main.yes(): boolean {
entry:
    v0: int64 = 1
    v1: int64 = 2
    v2: boolean = lt v0, v1
    return v2
}
"#,
    );
}
