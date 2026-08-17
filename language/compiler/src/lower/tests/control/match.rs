use crate::tests::TestSession;

#[test]
fn test_lower_match_destructures_the_narrowed_union_member() {
    let session = TestSession::single(
        r#"
struct Off {
    kind: "off" = "off";
    code: int32;
}

struct On {
    kind: "on" = "on";
    level: int32;
}

newtype State = Off | On;

function read(state: State): int32 {
    match (state) {
        Off { code } => code
        On { level } => level
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Off {
    kind: void;
    code: int32;
}

@copy
type On {
    kind: void;
    level: int32;
}

@copy
type State = newtype<variant<uint1> { 0uint1 = Off; 1uint1 = On; }>;

function test.main.read(v0: State): int32 {
    local l0: int32

entry(v0: State):
    v1: variant<uint1> { 0uint1 = Off; 1uint1 = On; } = field.get v0, 0
    variant.switch v1, 0 => b1, 1 => b2

b1:
    v2: Off = variant.payload v1, 0
    v3: int32 = field.get v2, 1
    local.set l0, v3
    jump b3

b2:
    v4: On = variant.payload v1, 1
    v5: int32 = field.get v4, 1
    local.set l0, v5
    jump b3

b3:
    v6: int32 = local.get l0
    return v6
}

/// @layout.struct name=Off size=4 align=4
/// @layout.field owner=Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=On size=4 align=4
/// @layout.field owner=On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=4
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=4
"#,
    );
}
