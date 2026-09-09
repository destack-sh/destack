use crate::tests::TestSession;

#[test]
fn test_lower_field_read_to_field_get() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function sum(point: Point): int32 {
    return point.x + point.y;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.sum",
        r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.sum(v0: test.main.Point): int32 {
    local l0: test.main.Point

entry(v0: test.main.Point):
    local.set l0, v0
    v1: test.main.Point = local.get l0
    v2: int32 = field.get v1, 0
    v3: test.main.Point = local.get l0
    v4: int32 = field.get v3, 1
    v5: int32 = add v2, v4
    return v5
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_nested_field_read_chain() {
    let session = TestSession::single(
        r#"
struct Size {
    width: int32;
    height: int32;
}

struct Frame {
    corner: int32;
    size: Size;
}

function area(frame: Frame): int32 {
    return frame.size.width * frame.size.height;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.area",
        r#"
@copy
type test.main.Size {
    width: int32;
    height: int32;
}

@copy
type test.main.Frame {
    corner: int32;
    size: test.main.Size;
}

function test.main.area(v0: test.main.Frame): int32 {
    local l0: test.main.Frame

entry(v0: test.main.Frame):
    local.set l0, v0
    v1: test.main.Frame = local.get l0
    v2: test.main.Size = field.get v1, 1
    v3: int32 = field.get v2, 0
    v4: test.main.Frame = local.get l0
    v5: test.main.Size = field.get v4, 1
    v6: int32 = field.get v5, 1
    v7: int32 = mul v3, v6
    return v7
}

/// @layout.struct name=test.main.Size size=8 align=4
/// @layout.field owner=test.main.Size index=0 name=width offset=0 size=4 align=4
/// @layout.field owner=test.main.Size index=1 name=height offset=4 size=4 align=4
/// @layout.struct name=test.main.Frame size=12 align=4
/// @layout.field owner=test.main.Frame index=0 name=corner offset=0 size=4 align=4
/// @layout.field owner=test.main.Frame index=1 name=size offset=4 size=8 align=4
"#,
    );
}

#[test]
fn test_lower_a_borrow_of_a_member_place() {
    let session = TestSession::single(
        r#"
struct Meter {
    name: string;
}

function read(meter: &readonly Meter): &readonly string {
    return &readonly meter.name;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.read", r#"
@languageItem("string.String")
type String;

@copy
type test.main.Meter {
    name: ref<String, managed, mutable, local>;
}

function test.main.read<'a>(v0: ref<test.main.Meter, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.Meter, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Meter, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Meter, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: ref<String, managed, readonly, local> = load v2
    v4: ref<String, borrowed, 'a, readonly, local> = cast.bit v3 -> ref<String, borrowed, 'a, readonly, local>
    return v4
}

/// @layout.struct name=test.main.Meter size=8 align=8
/// @layout.field owner=test.main.Meter index=0 name=name offset=0 size=8 align=8
"#);
}

#[test]
fn test_lower_a_coalesced_handle_field_borrow_through_the_receiver() {
    let session = TestSession::single(
        r#"
struct Box {
    message?: string;
}

export extension of Box {
    implicit(): &readonly string {
        return this.message ?? "x";
    }

    explicit(&readonly this): &readonly string {
        return this.message ?? "x";
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds", r#"
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable, local>;
}

@copy
type test.main.Box {
    message: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

constant string.0: String = "x"

function test.main.Box.implicit<'a>(v0: ref<test.main.Box, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.Box, borrowed, 'a, readonly, local>
    local l1: ref<String, managed, readonly, local>, readonly

entry(v0: ref<test.main.Box, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Box, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; } = load v2
    variant.switch v3, 0 => b2, else b1

b1:
    v4: ref<String, managed, readonly, local> = variant.payload v3, 1
    local.set l1, v4
    jump b3

b2:
    v5: ref<String, managed, mutable, local> = global.address string.0
    local.set l1, v5
    jump b3

b3:
    v6: ref<String, managed, readonly, local> = local.get l1
    v7: ref<String, borrowed, 'a, readonly, local> = cast.bit v6 -> ref<String, borrowed, 'a, readonly, local>
    return v7
}

function test.main.Box.explicit<'a>(v0: ref<test.main.Box, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.Box, borrowed, 'a, readonly, local>
    local l1: ref<String, managed, readonly, local>, readonly

entry(v0: ref<test.main.Box, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Box, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; } = load v2
    variant.switch v3, 0 => b2, else b1

b1:
    v4: ref<String, managed, readonly, local> = variant.payload v3, 1
    local.set l1, v4
    jump b3

b2:
    v5: ref<String, managed, mutable, local> = global.address string.0
    local.set l1, v5
    jump b3

b3:
    v6: ref<String, managed, readonly, local> = local.get l1
    v7: ref<String, borrowed, 'a, readonly, local> = cast.bit v6 -> ref<String, borrowed, 'a, readonly, local>
    return v7
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=test.main.Box size=8 align=8
/// @layout.field owner=test.main.Box index=0 name=message offset=0 size=8 align=8
/// @layout.variant name=type@9 size=8 align=8
/// @layout.discriminant owner=type@9 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@9 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@9 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@21 size=8 align=8
/// @layout.discriminant owner=type@21 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@21 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@21 index=1 discriminant=1 payload_offset=0
"#,
    );
    session.assert_mir_function("main.ds", "test.main.Box.implicit", r#"
@languageItem("string.String")
type String;

@copy
type test.main.Box {
    message: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

function test.main.Box.implicit<'a>(v0: ref<test.main.Box, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.Box, borrowed, 'a, readonly, local>
    local l1: ref<String, managed, readonly, local>, readonly

entry(v0: ref<test.main.Box, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Box, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; } = load v2
    variant.switch v3, 0 => b2, else b1

b1:
    v4: ref<String, managed, readonly, local> = variant.payload v3, 1
    local.set l1, v4
    jump b3

b2:
    v5: ref<String, managed, mutable, local> = global.address string.0
    local.set l1, v5
    jump b3

b3:
    v6: ref<String, managed, readonly, local> = local.get l1
    v7: ref<String, borrowed, 'a, readonly, local> = cast.bit v6 -> ref<String, borrowed, 'a, readonly, local>
    return v7
}

/// @layout.struct name=test.main.Box size=8 align=8
/// @layout.field owner=test.main.Box index=0 name=message offset=0 size=8 align=8
/// @layout.variant name=type@21 size=8 align=8
/// @layout.discriminant owner=type@21 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@21 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@21 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.ds", "test.main.Box.explicit", r#"
@languageItem("string.String")
type String;

@copy
type test.main.Box {
    message: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

function test.main.Box.explicit<'a>(v0: ref<test.main.Box, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.Box, borrowed, 'a, readonly, local>
    local l1: ref<String, managed, readonly, local>, readonly

entry(v0: ref<test.main.Box, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Box, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; } = load v2
    variant.switch v3, 0 => b2, else b1

b1:
    v4: ref<String, managed, readonly, local> = variant.payload v3, 1
    local.set l1, v4
    jump b3

b2:
    v5: ref<String, managed, mutable, local> = global.address string.0
    local.set l1, v5
    jump b3

b3:
    v6: ref<String, managed, readonly, local> = local.get l1
    v7: ref<String, borrowed, 'a, readonly, local> = cast.bit v6 -> ref<String, borrowed, 'a, readonly, local>
    return v7
}

/// @layout.struct name=test.main.Box size=8 align=8
/// @layout.field owner=test.main.Box index=0 name=message offset=0 size=8 align=8
/// @layout.variant name=type@21 size=8 align=8
/// @layout.discriminant owner=type@21 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@21 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@21 index=1 discriminant=1 payload_offset=0
"#);
}
