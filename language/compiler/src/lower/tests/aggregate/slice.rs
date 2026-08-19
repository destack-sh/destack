use crate::tests::TestSession;

#[test]
fn test_lower_borrowed_slice_parameters_to_fat_descriptors() {
    let session = TestSession::single(
        r#"
function measure(values: &readonly [int32]): int32 {
    return 7;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.measure<'a>(v0: slice<int32, borrowed, 'a, readonly>): int32 {
entry(v0: slice<int32, borrowed, 'a, readonly>):
    v1: int32 = 7
    return v1
}
"#,
    );
}

#[test]
fn test_lower_string_parameters_through_the_representation_class() {
    let session = TestSession::single(
        r#"
function keep(name: string): string {
    return name;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type destack.string.string.String {
    codeUnits: slice<uint16, unique, exclusive>;
}

function test.main.keep(v0: ref<destack.string.string.String, managed, mutable>): ref<destack.string.string.String, managed, mutable> {
entry(v0: ref<destack.string.string.String, managed, mutable>):
    return v0
}

/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}

#[test]
fn test_lower_newtype_slice_borrows_to_fat_descriptors() {
    let session = TestSession::single(
        r#"
newtype Bytes = [uint8];

function measure(view: &readonly Bytes): int32 {
    return 7;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Bytes = newtype<slice<uint8, managed, mutable>>;

function test.main.measure<'a>(v0: slice<uint8, borrowed, 'a, readonly>): int32 {
entry(v0: slice<uint8, borrowed, 'a, readonly>):
    v1: int32 = 7
    return v1
}
"#,
    );
}
