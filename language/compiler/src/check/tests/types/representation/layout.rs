use crate::tests::{DirRows, TestSession};

#[test]
fn test_layout_queries_return_static_values() {
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
=== annotated ===
struct Header {
    tag: uint8;
    size: uint32;
}

const size: usize = comptime sizeOf<Header>();
const alignment: usize = comptime alignOf<Header>();
const stride: usize = comptime strideOf<Header>();

=== checked ===
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
fn test_size_of_participates_in_static_inference() {
    let session = TestSession::single(
        r#"
struct Header {
    tag: uint8;
    size: uint32;
}

declare function length<T, comptime N: usize>(values: [T; N]): N;

declare let bytes: [uint8; sizeOf<Header>()];
const bytesLength = length(bytes);

bytesLength satisfies sizeOf<Header>();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics().with_layout(),
        r#"
=== annotated ===
struct Header {
    tag: uint8;
    size: uint32;
}

declare function length<T, comptime N: usize>(values: [T; N]): N;

declare let bytes: [uint8; sizeOf<Header>()];
const bytesLength: sizeOf<Header>() = length<uint8, sizeOf<Header>()>(bytes);

bytesLength satisfies sizeOf<Header>();

=== checked ===
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

declare function length<T, comptime N: usize>(values: [T; N]): N;
/// @generic.template symbol=length parameters=(T, comptime N: usize)
/// @type.symbol symbol=length type=<T, comptime N: usize>([T; N]) => N
/// @type.symbol symbol=values type=[T; N]

declare let bytes: [uint8; sizeOf<Header>()];
/// @type.symbol symbol=bytes source=bytes type=[uint8; sizeOf<Header>()]
/// @resolution.name source=Header target=Header
/// @static.node source=sizeOf<Header>() value=8

const bytesLength = length(bytes);
/// @type.symbol symbol=bytesLength type=sizeOf<Header>()
/// @resolution.name source=length target=length
/// @resolution.name source=bytes target=bytes
/// @resolution.call source=length(bytes) parameters=([uint8; 8]) return=8 kind=symbol target=length instance="length<uint8, 8>"
/// @generic.instance source=length(bytes) id="length<uint8, 8>"

bytesLength satisfies sizeOf<Header>();
/// @resolution.name source=bytesLength target=bytesLength
/// @resolution.name source=Header target=Header
/// @static.node source=sizeOf<Header>() value=8
/// @generic.instance id="length<uint8, 8>" template=length arguments=(uint8, 8)
"#,
    );
}

#[test]
fn test_stride_of_participates_in_static_defaults() {
    let session = TestSession::single(
        r#"
struct Header {
    tag: uint8;
    size: uint32;
}

type Slots<T: Concrete, comptime N: usize = strideOf<T>()> = [uint8; N];

declare let slots: Slots<Header>;

slots satisfies [uint8; strideOf<Header>()];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics().with_layout(),
        r#"
=== annotated ===
struct Header {
    tag: uint8;
    size: uint32;
}

type Slots<T: Concrete, comptime N: usize = strideOf<T>()> = [uint8; N];

declare let slots: Slots<Header>;

slots satisfies [uint8; strideOf<Header>()];

=== checked ===
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

type Slots<T: Concrete, comptime N: usize = strideOf<T>()> = [uint8; N];
/// @generic.template symbol=Slots parameters=(T: Concrete, comptime N: usize = strideOf<T>())
/// @type.symbol symbol=Slots source="type Slots<T: Concrete, comptime N: usize = strideOf<T>()> = [uint8; N]" type=[uint8; N]
/// @definition.type symbol=Slots source="type Slots<T: Concrete, comptime N: usize = strideOf<T>()> = [uint8; N]" template=LocalGenericTemplateId(0) value=[uint8; N]
/// @type.symbol symbol=Slots.T source=T type=T
/// @type.symbol symbol=Slots.N source=N type=N
/// @resolution.name source=Concrete target=memory.Concrete
/// @resolution.name source=T target=Slots.T
/// @resolution.name source=N target=Slots.N

declare let slots: Slots<Header>;
/// @type.symbol symbol=slots source=slots type=Slots<Header, strideOf<Header>()>
/// @resolution.name source=Slots target=Slots
/// @resolution.name source=Header target=Header
/// @static.node source=strideOf<Header>() value=8
/// @generic.instance source="Slots<Header>" id="Slots<Header, 8>"

slots satisfies [uint8; strideOf<Header>()];
/// @resolution.name source=slots target=slots
/// @resolution.name source=Header target=Header
/// @static.node source=strideOf<Header>() value=8
/// @generic.instance id="Slots<Header, 8>" template=Slots arguments=(Header, 8)
"#,
    );
}

#[test]
fn test_layout_of_returns_reflected_shape() {
    let session = TestSession::single(
        r#"
struct Header {
    tag: uint8;
    size: uint32;
}

const layout = comptime layoutOf<Header>();

layout satisfies Layout;
layout.shape satisfies { kind: "aggregate"; fields: readonly LayoutField[] };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics().with_layout(),
        r#"
=== annotated ===
struct Header {
    tag: uint8;
    size: uint32;
}

const layout: Layout = comptime layoutOf<Header>();

layout satisfies Layout;
layout.shape satisfies { kind: "aggregate"; fields: readonly LayoutField[] };

=== checked ===
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

const layout = comptime layoutOf<Header>();
/// @resolution.name source=Header target=Header
/// @type.symbol symbol=layout type=Layout
/// @static.symbol symbol=layout value=Layout { type: Header }

layout satisfies Layout;
/// @resolution.name source=layout target=layout
/// @resolution.name source=Layout target=memory.layout.Layout

layout.shape satisfies { kind: "aggregate"; fields: readonly LayoutField[] };
/// @resolution.name source=layout target=layout
/// @resolution.member source=layout.shape receiver=Layout kind=field key=shape
/// @resolution.name source=LayoutField target=memory.layout.LayoutField
"#,
    );
}

#[test]
fn test_representation_attributes_set_layout_metadata() {
    let session = TestSession::single(
        r#"
@repr("transparent")
newtype FileDescriptor = int32;

@repr("C", { packed: true })
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
=== annotated ===
@repr("transparent")
newtype FileDescriptor = int32;

@repr("C", { packed: true })
struct WireHeader {
    tag: uint8;
    size: uint32;
}

=== checked ===
@repr("transparent")
/// @type.symbol symbol=FileDescriptor type=FileDescriptor
/// @layout.type type=FileDescriptor shape=newtype size=4 align=4 backing=scalar(4/4)
/// @definition.newtype symbol=FileDescriptor source="newtype FileDescriptor = int32" value=int32

newtype FileDescriptor = int32;

@repr("C", { packed: true })
/// @type.symbol symbol=WireHeader type=WireHeader
/// @layout.type type=WireHeader shape=struct size=5 align=1
/// @layout.field parent=WireHeader key=size type=uint32 offset=1 size=4 align=1
/// @layout.field parent=WireHeader key=tag type=uint8 offset=0 size=1 align=1
/// @definition.field symbol=WireHeader.size source="size: uint32" key=size type=uint32
/// @definition.field symbol=WireHeader.tag source="tag: uint8" key=tag type=uint8
/// @definition.struct symbol=WireHeader

struct WireHeader {
    tag: uint8;
    /// @type.symbol symbol=WireHeader.tag source="tag: uint8" type=uint8

    size: uint32;
    /// @type.symbol symbol=WireHeader.size source="size: uint32" type=uint32

}

/// @static.entry value="\"C\""
/// @static.entry value="\"transparent\""
/// @static.entry value="{ packed: true }"

"#,
    );
}

#[test]
fn test_representation_attributes_set_alignment_and_enum_backing() {
    let session = TestSession::single(
        r#"
@repr({ align: 16 })
struct Block {
    value: uint8;
}

@repr("uint8")
enum Mode {
    read,
    write,
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics().with_layout(),
        r#"
=== annotated ===
@repr({ align: 16 })
struct Block {
    value: uint8;
}

@repr("uint8")
enum Mode {
    read,
    write,
}

=== checked ===
@repr({ align: 16 })
/// @type.symbol symbol=Block type=Block
/// @layout.type type=Block shape=struct size=16 align=16
/// @layout.field parent=Block key=value type=uint8 offset=0 size=1 align=1
/// @definition.field symbol=Block.value source="value: uint8" key=value type=uint8
/// @definition.struct symbol=Block

struct Block {
    value: uint8;
    /// @type.symbol symbol=Block.value source="value: uint8" type=uint8
}

@repr("uint8")
/// @type.symbol symbol=Mode type=Mode
/// @layout.type type=Mode shape=enum size=1 align=1 backing=uint8

enum Mode {
    read,
    /// @definition.enumMember symbol=Mode.read key=read

    write,
    /// @definition.enumMember symbol=Mode.write key=write
}

/// @static.entry value="{ align: 16 }"
/// @static.entry value="\"uint8\""
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
=== annotated ===
type Writer = {
    write(bytes: uint8[]): uint;
};

const size = comptime sizeOf<Writer>();

=== checked ===
type Writer = {
/// @type.symbol symbol=Writer type={ write(uint8[]) => uint }

    write(bytes: uint8[]): uint;
};

const size = comptime sizeOf<Writer>();
/// @resolution.name source=Writer target=Writer

"#,
        r#"
/// @diagnostic.error code=EC500 message="type '{ write(uint8[]) => uint }' has no concrete layout"
/// @diagnostic.label line=6 column=23 source="const size = comptime sizeOf<Writer>();"
"#,
    );
}
