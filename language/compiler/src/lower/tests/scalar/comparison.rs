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
        "main.tspp",
        "test.main.less",
        r#"
export function test.main.less(v0: int32, v1: int32): boolean {
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
        "main.tspp",
        "test.main.above",
        r#"
export function test.main.above(v0: uint32, v1: uint32): boolean {
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
        "main.tspp",
        "test.main.same",
        r#"
export function test.main.same(v0: int64, v1: int64): boolean {
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
        "main.tspp",
        "test.main.yes",
        r#"
export function test.main.yes(): boolean {
entry:
    v0: int64 = 1
    v1: int64 = 2
    v2: boolean = lt v0, v1
    return v2
}
"#,
    );
}

/// Compare strings by content through the language equality function.
#[test]
fn test_lower_string_equality_to_content_comparison() {
    let session = TestSession::single(
        r#"
function same(left: string, right: string): boolean {
    left === right || left != right
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.same",
        r#"
@nocopy
@languageItem("string.String")
type String;

export function test.main.same(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>): boolean {
    local l0: ref<String, managed, mutable, local>
    local l1: ref<String, managed, mutable, local>
    local l2: boolean

entry(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: ref<String, managed, mutable, local> = load l0
    v3: ref<String, managed, mutable, local> = load l1
    v4: boolean = call stringEqual(v2, v3): (ref<String, managed, mutable, local>, ref<String, managed, mutable, local>) => boolean
    store l2, v4
    branch v4 => b2 | b1

b1:
    v5: ref<String, managed, mutable, local> = load l0
    v6: ref<String, managed, mutable, local> = load l1
    v7: boolean = call stringEqual(v5, v6): (ref<String, managed, mutable, local>, ref<String, managed, mutable, local>) => boolean
    v8: boolean = not v7
    store l2, v8
    jump b2

b2:
    v9: boolean = load l2
    return v9
}
"#,
    );
}
