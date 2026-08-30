use crate::tests::TestSession;

/// Read a string literal from the constant String object declared for its content.
#[test]
fn test_lower_string_literal_to_constant_object_read() {
    let session = TestSession::single(
        r#"
function greet(): string {
    return "hello 😀";
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

constant string.0: String = "hello \u{1f600}"

function test.main.greet(): ref<String, managed, mutable, local> {
entry:
    v0: ref<String, managed, mutable, local> = global.address string.0
    return v0
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}

/// Read repeated string literals of one content from a single constant String object.
#[test]
fn test_intern_repeated_string_literals_into_one_object() {
    let session = TestSession::single(
        r#"
function pair(): string {
    let first = "hi";
    return "hi";
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

constant string.0: String = "hi"

function test.main.pair(): ref<String, managed, mutable, local> {
    local l0: ref<String, managed, mutable, local>

entry:
    v0: ref<String, managed, mutable, local> = global.address string.0
    local.set l0, v0
    v1: ref<String, managed, mutable, local> = global.address string.0
    return v1
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}
