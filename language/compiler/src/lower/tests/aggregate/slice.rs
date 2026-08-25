use crate::tests::TestSession;

/// Lower a borrowed slice parameter to a fat descriptor.
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
function test.main.measure<'a>(v0: slice<int32, borrowed, 'a, readonly, local>): int32 {
entry(v0: slice<int32, borrowed, 'a, readonly, local>):
    v1: int32 = 7
    return v1
}
"#,
    );
}

/// Lower a string parameter through the String representation class.
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
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

function test.main.keep(v0: ref<String, managed, mutable, local>): ref<String, managed, mutable, local> {
entry(v0: ref<String, managed, mutable, local>):
    return v0
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}

/// Lower a borrow of a slice newtype to a fat descriptor.
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
type Bytes = newtype<slice<uint8, managed, mutable, local>>;

function test.main.measure<'a>(v0: slice<uint8, borrowed, 'a, readonly, local>): int32 {
entry(v0: slice<uint8, borrowed, 'a, readonly, local>):
    v1: int32 = 7
    return v1
}
"#,
    );
}
