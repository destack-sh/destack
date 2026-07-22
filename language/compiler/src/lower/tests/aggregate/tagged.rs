use crate::tests::TestSession;

#[test]
fn test_lower_scalar_union_entries_to_tagged_variants() {
    let session = TestSession::single(
        r#"
function pick(flag: boolean, count: int32): int32 | boolean {
    if (flag) {
        return count;
    }
    return 5;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.pick(v0: boolean, v1: int32): variant<uint8, int32> { 0uint8 = int32; 1uint8 = boolean; } {
entry(v0: boolean, v1: int32):
    branch v0, b1, b2

b1:
    v2: variant<uint8, int32> { 0uint8 = int32; 1uint8 = boolean; } = variant.new 0, v1
    return v2

b2:
    v3: int32 = 5
    v4: variant<uint8, int32> { 0uint8 = int32; 1uint8 = boolean; } = variant.new 0, v3
    return v4
}
/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_tagged_newtype_to_a_variant_carrier() {
    let session = TestSession::single(
        r#"
struct Circle {
    kind: "circle";
    radius: float64;
}

struct Square {
    kind: "square";
    side: int32;
}

@derive(Tagged)
newtype Shape = Circle | Square;

function keep(shape: Shape): Shape {
    return shape;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Circle {
    kind: void;
    radius: float64;
}

@copy
type Square {
    kind: void;
    side: int32;
}

@copy
type Shape = variant<uint8, Circle> { 0uint8 = Circle; 1uint8 = Square; };

function main.keep(v0: Shape): Shape {
entry(v0: Shape):
    return v0
}
/// @layout.struct name=Circle size=8 align=8
/// @layout.field owner=Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=Square size=4 align=4
/// @layout.field owner=Square index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Square index=1 name=side offset=0 size=4 align=4
/// @layout.variant name=Shape size=16 align=8
/// @layout.discriminant owner=Shape kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=Shape index=0 discriminant=0 payload_offset=8
/// @layout.case owner=Shape index=1 discriminant=1 payload_offset=8
"#,
    );
}

#[test]
fn test_lower_boolean_undefined_union_into_a_niche() {
    let session = TestSession::single(
        r#"
function keep(value: boolean | undefined): boolean | undefined {
    return value;
}

function forget(): boolean | undefined {
    return undefined;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.keep(v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; }): variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; } {
entry(v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; }):
    return v0
}

function main.forget(): variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; } {
entry:
    v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; } = variant.new 1
    return v0
}
/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_cases=1..1 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#,
    );
}
