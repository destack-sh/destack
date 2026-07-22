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
type Mode = variant<int64, void> { 1int64 = void; 2int64 = void; };

function main.pick(v0: boolean): Mode {
entry(v0: boolean):
    branch v0, b1, b2

b1:
    v1: Mode = variant.new 0
    return v1

b2:
    v2: Mode = variant.new 1
    return v2
}
/// @layout.variant name=Mode size=8 align=8 encoding=direct(tag@0+8) cases=(1@8, 2@8)
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
type Mode = variant<int64, void> { 1int64 = void; 2int64 = void; };

function main.describe(v0: Mode): int32 {
    local l0: int32

entry(v0: Mode):
    variant.switch v0, 0 => b1, 1 => b2

b1:
    v1: int32 = 10
    local.set l0, v1
    jump b3

b2:
    v2: int32 = 20
    local.set l0, v2
    jump b3

b3:
    v3: int32 = local.get l0
    return v3
}

function main.fallback(v0: Mode): int32 {
    local l0: int32

entry(v0: Mode):
    variant.switch v0, 0 => b1, else b2

b1:
    v1: int32 = 10
    local.set l0, v1
    jump b3

b2:
    v2: int32 = 0
    local.set l0, v2
    jump b3

b3:
    v3: int32 = local.get l0
    return v3
}
/// @layout.variant name=Mode size=8 align=8 encoding=direct(tag@0+8) cases=(1@8, 2@8)
"#,
    );
}
