use crate::tests::TestSession;

#[test]
fn test_lower_enum_members_to_variant_cases() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function pick(flag: boolean): Mode {
    if (flag) {
        return Mode.Read;
    }
    return Mode.Write;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

function test.main.pick(v0: boolean): test.main.Mode {
    local l0: boolean

entry(v0: boolean):
    store l0, v0
    v1: boolean = load l0
    branch v1 => b1 | b2

b1:
    v2: test.main.Mode = variant.new 0
    return v2

b2:
    v3: test.main.Mode = variant.new 1
    return v3
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
"#,
    );
}

#[test]
fn test_lower_enum_match_to_a_variant_switch() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): int32 {
    match (mode) {
        Mode.Read => 10
        Mode.Write => 20
    }
}

function fallback(mode: Mode): int32 {
    match (mode) {
        Mode.Read => 10
        _ => 0
    }
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.describe",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

function test.main.describe(v0: test.main.Mode): int32 {
    local l0: test.main.Mode
    local l1: int32

entry(v0: test.main.Mode):
    store l0, v0
    v1: uint8 = variant.tag.load l0
    switch v1, b3, 0 => b1, 1 => b2

b1:
    v2: int32 = 10
    store l1, v2
    jump b4

b2:
    v3: int32 = 20
    store l1, v3
    jump b4

b3:
    unreachable

b4:
    v4: int32 = load l1
    return v4
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.fallback",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

function test.main.fallback(v0: test.main.Mode): int32 {
    local l0: test.main.Mode
    local l1: int32

entry(v0: test.main.Mode):
    store l0, v0
    v1: uint8 = variant.tag.load l0
    switch v1, b2, 0 => b1

b1:
    v2: int32 = 10
    store l1, v2
    jump b4

b2:
    v3: int32 = 0
    store l1, v3
    jump b4

b3:
    unreachable

b4:
    v4: int32 = load l1
    return v4
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
"#,
    );
}
