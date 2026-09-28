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
export function test.main.advance(v0: uint32): uint32 {
    local l0: uint32

entry(v0: uint32):
    store l0, v0
    v1: uint32 = load l0
    v2: uint32 = 122
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: uint32 = 97
    return v4

b2:
    v5: uint32 = load l0
    return v5
}
"#,
    );
}
