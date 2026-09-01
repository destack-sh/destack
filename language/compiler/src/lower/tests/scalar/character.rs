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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.advance(v0: uint32): uint32 {
entry(v0: uint32):
    v1: uint32 = 122
    v2: boolean = eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: uint32 = 97
    return v3

b2:
    return v0
}
"#,
    );
}
