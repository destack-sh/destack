use crate::tests::TestSession;

/// A defaulted literal union accepts a supplied value or evaluates its default.
#[test]
fn test_call_defaulted_literal_union() {
    let session = TestSession::single(
        r#"
type Order = "big" | "little";

function order(value: Order = "big"): Order {
    return value;
}

function read(): Order {
    return order("little");
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type literal.string.big { }

type literal.string.little { }

@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

constant string.0: String = "big"
constant string.1: String = "little"

function test.main.order(v0: variant<uint2> { 0uint2 = literal.string.big; 1uint2 = literal.string.little; 2uint2 = void; }): ref<String, managed, mutable, local> {
    local l0: variant<uint2> { 0uint2 = literal.string.big; 1uint2 = literal.string.little; 2uint2 = void; }
    local l1: ref<String, managed, mutable, local>, readonly
    local l2: ref<String, managed, mutable, local>
    local l3: ref<String, managed, mutable, local>

entry(v0: variant<uint2> { 0uint2 = literal.string.big; 1uint2 = literal.string.little; 2uint2 = void; }):
    store l0, v0
    v1: variant<uint2> { 0uint2 = literal.string.big; 1uint2 = literal.string.little; 2uint2 = void; } = load l0
    variant.switch v1, 2 => b2, else b1

b1:
    variant.switch v1, 0 => b6, 1 => b7, else b5

b2:
    v5: ref<String, managed, mutable, local> = address @string.0
    store l1, v5
    jump b3

b3:
    v6: ref<String, managed, mutable, local> = load l1
    store l3, v6
    v7: ref<String, managed, mutable, local> = load l3
    return v7

b4:
    v4: ref<String, managed, mutable, local> = load l2
    store l1, v4
    jump b3

b5:
    panic

b6:
    v2: ref<String, managed, mutable, local> = address @string.0
    store l2, v2
    jump b4

b7:
    v3: ref<String, managed, mutable, local> = address @string.1
    store l2, v3
    jump b4
}

function test.main.read(): ref<String, managed, mutable, local> {
entry:
    v0: literal.string.little = zeroed
    v1: variant<uint2> { 0uint2 = literal.string.big; 1uint2 = literal.string.little; 2uint2 = void; } = variant.new 1
    v2: ref<String, managed, mutable, local> = call test.main.order(v1): (variant<uint2> { 0uint2 = literal.string.big; 1uint2 = literal.string.little; 2uint2 = void; }) => ref<String, managed, mutable, local>
    return v2
}

/// @layout.struct name=literal.string.big size=0 align=1
/// @layout.struct name=literal.string.little size=0 align=1
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@1 size=0 align=1
/// @layout.variant name=type@5 size=1 align=1
/// @layout.discriminant owner=type@5 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@5 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@5 index=1 discriminant=1 payload_offset=1
/// @layout.case owner=type@5 index=2 discriminant=2 payload_offset=1
/// @layout.struct name=type@10 size=16 align=8
/// @layout.field owner=type@10 index=0 name=codeUnits offset=0 size=16 align=8
"#);
}

#[test]
fn test_lower_omitted_call_evaluates_the_callee_default() {
    let session = TestSession::single(
        r#"
function greet(count: int32 = 3): int32 {
    return count;
}

function main(): int32 {
    return greet();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.greet",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): int32 {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    local l1: int32, readonly
    local l2: int32

entry(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l0
    variant.switch v1, 1 => b2, else b1

b1:
    v2: int32 = variant.payload v1, 0
    store l1, v2
    jump b3

b2:
    v3: int32 = 3
    store l1, v3
    jump b3

b3:
    v4: int32 = load l1
    store l2, v4
    v5: int32 = load l2
    return v5
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.main", r#"
function test.main.main(): int32 {
entry:
    v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    v1: int32 = call test.main.greet(v0): (variant<uint1> { 0uint1 = int32; 1uint1 = void; }) => int32
    return v1
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_provided_call_wraps_the_defaulted_parameter() {
    let session = TestSession::single(
        r#"
function greet(count: int32 = 3): int32 {
    return count;
}

function main(): int32 {
    return greet(7);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.greet",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): int32 {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    local l1: int32, readonly
    local l2: int32

entry(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l0
    variant.switch v1, 1 => b2, else b1

b1:
    v2: int32 = variant.payload v1, 0
    store l1, v2
    jump b3

b2:
    v3: int32 = 3
    store l1, v3
    jump b3

b3:
    v4: int32 = load l1
    store l2, v4
    v5: int32 = load l2
    return v5
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.main", r#"
function test.main.main(): int32 {
entry:
    v0: int32 = 7
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    v2: int32 = call test.main.greet(v1): (variant<uint1> { 0uint1 = int32; 1uint1 = void; }) => int32
    return v2
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_undefined_argument_takes_the_callee_default() {
    let session = TestSession::single(
        r#"
function greet(count: int32 | undefined = 3): int32 {
    return count;
}

function main(): int32 {
    return greet(undefined);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.greet",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): int32 {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    local l1: int32, readonly
    local l2: int32

entry(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l0
    variant.switch v1, 1 => b2, else b1

b1:
    v2: int32 = variant.payload v1, 0
    store l1, v2
    jump b3

b2:
    v3: int32 = 3
    store l1, v3
    jump b3

b3:
    v4: int32 = load l1
    store l2, v4
    v5: int32 = load l2
    return v5
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.main", r#"
function test.main.main(): int32 {
entry:
    v0: void = zeroed
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    v2: int32 = call test.main.greet(v1): (variant<uint1> { 0uint1 = int32; 1uint1 = void; }) => int32
    return v2
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
}
