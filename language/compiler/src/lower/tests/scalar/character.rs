use crate::tests::TestSession;

#[test]
fn test_lower_characters_to_unicode_scalar_values() {
    let session = TestSession::single(
        r#"
function advance(current: char): char {
    if (current == 'z') {
        return 'a';
    }

    return current;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.advance",
        r#"
type literal.character.z { }

function test.main.advance(v0: uint32): uint32 {
    local l0: uint32

entry(v0: uint32):
    store l0, v0
    v1: uint32 = load l0
    v2: literal.character.z = zeroed
    v3: uint32 = 122
    v4: boolean = eq v1, v3
    branch v4 => b1 | b2

b1:
    v5: uint32 = 97
    return v5

b2:
    v6: uint32 = load l0
    return v6
}

/// @layout.struct name=literal.character.z size=0 align=1
"#,
    );
}
