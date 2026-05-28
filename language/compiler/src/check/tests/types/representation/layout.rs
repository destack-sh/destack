use crate::tests::{DirRows, TestSession};

#[test]
fn test_layout_intrinsic_returns_static_layout_value() {
    let session = TestSession::single(
        r#"
struct Header {
    tag: uint8;
    size: uint32;
}

const size = comptime sizeOf<Header>();
const alignment = comptime alignOf<Header>();
const stride = comptime strideOf<Header>();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics().with_layout(),
        r#"
struct Header {
/// @type.symbol symbol=Header type=Header
/// @layout.type type=Header shape=struct size=8 align=4
/// @layout.field parent=Header key=tag type=uint8 offset=0 size=1 align=1
/// @layout.field parent=Header key=size type=uint32 offset=4 size=4 align=4

    tag: uint8;
    /// @type.symbol symbol=Header.tag type=uint8

    size: uint32;
    /// @type.symbol symbol=Header.size type=uint32
}

const size = comptime sizeOf<Header>();
/// @resolution.name source=Header target=Header
/// @type.symbol symbol=size type=usize
/// @static.symbol symbol=size value=8

const alignment = comptime alignOf<Header>();
/// @resolution.name source=Header target=Header
/// @type.symbol symbol=alignment type=usize
/// @static.symbol symbol=alignment value=4

const stride = comptime strideOf<Header>();
/// @resolution.name source=Header target=Header
/// @type.symbol symbol=stride type=usize
/// @static.symbol symbol=stride value=8

"#,
    );
}

#[test]
fn test_representation_attributes_set_layout_metadata() {
    let session = TestSession::single(
        r#"
@repr("transparent")
newtype FileDescriptor = int32;

@repr("C")
@packed
struct WireHeader {
    tag: uint8;
    size: uint32;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics().with_layout(),
        r#"
@repr("transparent")
newtype FileDescriptor = int32;
/// @type.symbol symbol=FileDescriptor type=FileDescriptor
/// @layout.type type=FileDescriptor shape=newtype size=4 align=4 backing=scalar(4/4)

@repr("C")
@packed
struct WireHeader {
/// @type.symbol symbol=WireHeader type=WireHeader
/// @layout.type type=WireHeader shape=struct size=5 align=1
/// @layout.field parent=WireHeader key=tag type=uint8 offset=0 size=1 align=1
/// @layout.field parent=WireHeader key=size type=uint32 offset=1 size=4 align=1

    tag: uint8;
    /// @type.symbol symbol=WireHeader.tag type=uint8

    size: uint32;
    /// @type.symbol symbol=WireHeader.size type=uint32
}

"#,
    );
}

#[test]
fn test_layout_query_on_transparent_constraint_reports_error() {
    let session = TestSession::single(
        r#"
type Writer = {
    write(bytes: uint8[]): uint;
};

const size = comptime sizeOf<Writer>();
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
type Writer = {
/// @type.symbol symbol=Writer type={ write(uint8[]) => uint }

    write(bytes: uint8[]): uint;
};

const size = comptime sizeOf<Writer>();
/// @resolution.name source=Writer target=Writer

"#,
        r#"
/// @diagnostic.error code=EC500 message="type has no concrete layout"
/// @diagnostic.label line=6 column=30 source="const size = comptime sizeOf<Writer>();"
"#,
    );
}
