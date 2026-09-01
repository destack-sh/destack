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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

function test.main.pick(v0: boolean): Mode {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: Mode = variant.new 0
    return v1

b2:
    v2: Mode = variant.new 1
    return v2
}

/// @layout.variant name=Mode size=1 align=1
/// @layout.discriminant owner=Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=Mode index=1 discriminant=2 payload_offset=1
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

function test.main.describe(v0: Mode): int32 {
    local l0: int32

entry(v0: Mode):
    variant.switch v0, 0 => b1, 1 => b2, else b3

b1:
    v1: int32 = 10
    local.set l0, v1
    jump b4

b2:
    v2: int32 = 20
    local.set l0, v2
    jump b4

b3:
    unreachable

b4:
    v3: int32 = local.get l0
    return v3
}

function test.main.fallback(v0: Mode): int32 {
    local l0: int32

entry(v0: Mode):
    variant.switch v0, 0 => b1, else b2

b1:
    v1: int32 = 10
    local.set l0, v1
    jump b4

b2:
    v2: int32 = 0
    local.set l0, v2
    jump b4

b3:
    unreachable

b4:
    v3: int32 = local.get l0
    return v3
}

/// @layout.variant name=Mode size=1 align=1
/// @layout.discriminant owner=Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=Mode index=1 discriminant=2 payload_offset=1
"#,
    );
}
