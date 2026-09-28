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
        "main.tspp",
        "test.main.add",
        r#"
export function test.main.add(v0: int32, v1: int32): int32 {
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
        "main.tspp",
        "test.main.calc",
        r#"
export function test.main.calc(v0: int32, v1: int32): int32 {
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
        "main.tspp",
        "test.main.split",
        r#"
export function test.main.split(v0: uint32, v1: uint32): uint32 {
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
        "main.tspp",
        "test.main.flip",
        r#"
export function test.main.flip(v0: int32): int32 {
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
        "main.tspp",
        "test.main.three",
        r#"
export function test.main.three(): int32 {
entry:
    v0: int32 = 3
    return v0
}
"#,
    );
}

/// Store interval and property key values at the scalar and string they range over.
#[test]
fn test_store_intervals_and_keys_at_their_carriers() {
    let session = TestSession::single(
        r#"
type Digit = 0..=9;
type Name = keyof { first: int32; last: int32 };

function next(digit: Digit): int64 {
    digit + 1
}

function pick(name: Name): Name {
    name
}
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

export function test.main.next(v0: int64): int64 {
    local l0: int64

entry(v0: int64):
    store l0, v0
    v1: int64 = load l0
    v2: int64 = 1
    v3: int64 = add v1, v2
    return v3
}

export function test.main.pick(v0: ref<String, managed, mutable, local>): ref<String, managed, mutable, local> {
    local l0: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, local>):
    store l0, v0
    v1: ref<String, managed, mutable, local> = load l0
    return v1
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@5 size=16 align=8
/// @layout.field owner=type@5 index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}
