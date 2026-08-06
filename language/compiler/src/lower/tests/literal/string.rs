use crate::tests::TestSession;

/// Read a string literal from the immortal String object declared for its content.
#[test]
fn test_lower_string_literal_to_immortal_object_read() {
    let session = TestSession::single(
        r#"
function greet(): string {
    return "hello";
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type destack.memory.unique.Unique<slice<uint8, managed, mutable>> = slice<uint8, unique, exclusive>;

type destack.string.string.String {
    bytes: destack.memory.unique.Unique<slice<uint8, managed, mutable>>;
}

immortal constant string.10557148580892020714.bytes: [uint8; 5] = b"hello"

immortal constant string.10557148580892020714: destack.string.string.String = {{globalAddress string.10557148580892020714.bytes, 5uint64}}

function test.main.greet(): ref<destack.string.string.String, managed, mutable> {
entry:
    v0: ref<destack.string.string.String, managed, mutable> = global.address string.10557148580892020714
    return v0
}
/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=bytes offset=0 size=16 align=8
"#,
    );
}

/// Read repeated string literals of one content from a single immortal String object.
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
type destack.memory.unique.Unique<slice<uint8, managed, mutable>> = slice<uint8, unique, exclusive>;

type destack.string.string.String {
    bytes: destack.memory.unique.Unique<slice<uint8, managed, mutable>>;
}

immortal constant string.13143504461344146821.bytes: [uint8; 2] = b"hi"

immortal constant string.13143504461344146821: destack.string.string.String = {{globalAddress string.13143504461344146821.bytes, 2uint64}}

function test.main.pair(): ref<destack.string.string.String, managed, mutable> {
    local l0: ref<destack.string.string.String, managed, mutable>

entry:
    v0: void = undefined
    local.set l0, v0
    v1: ref<destack.string.string.String, managed, mutable> = global.address string.13143504461344146821
    return v1
}
/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=bytes offset=0 size=16 align=8
"#,
    );
}
