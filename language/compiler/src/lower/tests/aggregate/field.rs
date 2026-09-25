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
        "main.tspp",
        "test.main.sum",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.sum(v0: test.main.Point): int32 {
    local l0: test.main.Point

entry(v0: test.main.Point):
    store l0, v0
    v1: int32 = load (l0).0
    v2: int32 = load (l0).1
    v3: int32 = add v1, v2
    return v3
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
        "main.tspp",
        "test.main.area",
        r#"
type test.main.Frame {
    corner: int32;
    size: test.main.Size;
}

function test.main.area(v0: test.main.Frame): int32 {
    local l0: test.main.Frame

entry(v0: test.main.Frame):
    store l0, v0
    v1: int32 = load ((l0).1).0
    v2: int32 = load ((l0).1).1
    v3: int32 = mul v1, v2
    return v3
}

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

    session.assert_mir_function("main.tspp", "test.main.read", r#"
type test.main.Meter {
    name: ref<String, managed, mutable, local>;
}

@nocopy
@languageItem("string.String")
type String;

function test.main.read<'a>(v0: ref<test.main.Meter, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Meter, borrowed, 'a, readonly>

entry(v0: ref<test.main.Meter, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Meter, borrowed, 'a, readonly> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v1).0
    v3: ref<String, managed, mutable, local> = load (*v2)
    v4: ref<String, borrowed, 'a, readonly> = cast.bit v3 -> ref<String, borrowed, 'a, readonly>
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
        "main.tspp", r#"
type test.main.Box {
    message: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

constant string.0: String = "x"

function test.main.Box.implicit<'a>(v0: ref<test.main.Box, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Box, borrowed, 'a, readonly>
    local l1: ref<String, borrowed, 'a, readonly>, readonly

entry(v0: ref<test.main.Box, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Box, borrowed, 'a, readonly> = load l0
    v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load (*v1).0
    variant.switch v2, 1 => b2, else b1

b1:
    v3: ref<String, managed, mutable, local> = variant.payload v2, 0
    store l1, v3
    jump b3

b2:
    v4: ref<String, managed, mutable, local> = address @string.0
    store l1, v4
    jump b3

b3:
    v5: ref<String, borrowed, 'a, readonly> = load l1
    return v5
}

function test.main.Box.explicit<'a>(v0: ref<test.main.Box, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Box, borrowed, 'a, readonly>
    local l1: ref<String, borrowed, 'a, readonly>, readonly

entry(v0: ref<test.main.Box, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Box, borrowed, 'a, readonly> = load l0
    v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load (*v1).0
    variant.switch v2, 1 => b2, else b1

b1:
    v3: ref<String, managed, mutable, local> = variant.payload v2, 0
    store l1, v3
    jump b3

b2:
    v4: ref<String, managed, mutable, local> = address @string.0
    store l1, v4
    jump b3

b3:
    v5: ref<String, borrowed, 'a, readonly> = load l1
    return v5
}

/// @layout.struct name=test.main.Box size=8 align=8
/// @layout.field owner=test.main.Box index=0 name=message offset=0 size=8 align=8
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@5 size=16 align=8
/// @layout.field owner=type@5 index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@9 size=8 align=8
/// @layout.field owner=type@9 index=0 name=message offset=0 size=8 align=8
"#,
    );
    session.assert_mir_function("main.tspp", "test.main.Box.implicit", r#"
type test.main.Box {
    message: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

function test.main.Box.implicit<'a>(v0: ref<test.main.Box, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Box, borrowed, 'a, readonly>
    local l1: ref<String, borrowed, 'a, readonly>, readonly

entry(v0: ref<test.main.Box, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Box, borrowed, 'a, readonly> = load l0
    v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load (*v1).0
    variant.switch v2, 1 => b2, else b1

b1:
    v3: ref<String, managed, mutable, local> = variant.payload v2, 0
    store l1, v3
    jump b3

b2:
    v4: ref<String, managed, mutable, local> = address @string.0
    store l1, v4
    jump b3

b3:
    v5: ref<String, borrowed, 'a, readonly> = load l1
    return v5
}

/// @layout.struct name=test.main.Box size=8 align=8
/// @layout.field owner=test.main.Box index=0 name=message offset=0 size=8 align=8
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.tspp", "test.main.Box.explicit", r#"
type test.main.Box {
    message: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

function test.main.Box.explicit<'a>(v0: ref<test.main.Box, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Box, borrowed, 'a, readonly>
    local l1: ref<String, borrowed, 'a, readonly>, readonly

entry(v0: ref<test.main.Box, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Box, borrowed, 'a, readonly> = load l0
    v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load (*v1).0
    variant.switch v2, 1 => b2, else b1

b1:
    v3: ref<String, managed, mutable, local> = variant.payload v2, 0
    store l1, v3
    jump b3

b2:
    v4: ref<String, managed, mutable, local> = address @string.0
    store l1, v4
    jump b3

b3:
    v5: ref<String, borrowed, 'a, readonly> = load l1
    return v5
}

/// @layout.struct name=test.main.Box size=8 align=8
/// @layout.field owner=test.main.Box index=0 name=message offset=0 size=8 align=8
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#);
}
