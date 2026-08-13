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
function test.main.pick(v0: boolean, v1: int32): variant<uint8> { 0uint8 = int32; 1uint8 = boolean; } {
entry(v0: boolean, v1: int32):
    branch v0 => b1 | b2

b1:
    v2: variant<uint8> { 0uint8 = int32; 1uint8 = boolean; } = variant.new 0, v1
    return v2

b2:
    v3: int32 = 5
    v4: variant<uint8> { 0uint8 = int32; 1uint8 = boolean; } = variant.new 0, v3
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
type Shape = variant<uint8> { 0uint8 = Circle; 1uint8 = Square; };

function test.main.keep(v0: Shape): Shape {
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
function test.main.keep(v0: variant<uint8> { 0uint8 = boolean; 1uint8 = void; }): variant<uint8> { 0uint8 = boolean; 1uint8 = void; } {
entry(v0: variant<uint8> { 0uint8 = boolean; 1uint8 = void; }):
    return v0
}

function test.main.forget(): variant<uint8> { 0uint8 = boolean; 1uint8 = void; } {
entry:
    v0: variant<uint8> { 0uint8 = boolean; 1uint8 = void; } = variant.new 1
    return v0
}
/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#,
    );
}

#[test]
fn test_lower_tagged_case_construction_to_variant_new() {
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

function make(radius: float64): Shape {
    return Shape.Circle({ radius });
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
type Shape = variant<uint8> { 0uint8 = Circle; 1uint8 = Square; };

function test.main.make(v0: float64): Shape {
entry(v0: float64):
    v1: { radius: float64 } = aggregate (v0)
    v2: ref<{ radius: float64 }, managed, mutable> = new.complete v1
    v3: Shape = variant.new 0, v2
    return v3
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
/// @layout.struct name=type@16 size=8 align=8
/// @layout.field owner=type@16 index=0 name=radius offset=0 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_payload_free_tagged_member() {
    let session = TestSession::single(
        r#"
struct Ready {
    state: "ready";
}

struct Pending {
    state: "pending";
}

@derive(Tagged)
newtype Status = Ready | Pending;

function pending(): Status {
    Status.Pending
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Ready {
    state: void;
}

@copy
type Pending {
    state: void;
}

@copy
type Status = variant<uint8> { 0uint8 = Ready; 1uint8 = Pending; };

function test.main.pending(): Status {
entry:
    v0: Status = variant.new 1
    return v0
}
/// @layout.struct name=Ready size=0 align=1
/// @layout.field owner=Ready index=0 name=state offset=0 size=0 align=1
/// @layout.struct name=Pending size=0 align=1
/// @layout.field owner=Pending index=0 name=state offset=0 size=0 align=1
/// @layout.variant name=Status size=1 align=1
/// @layout.discriminant owner=Status kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=Status index=0 discriminant=0 payload_offset=1
/// @layout.case owner=Status index=1 discriminant=1 payload_offset=1
"#,
    );
}

#[test]
fn test_construct_a_tagged_object_literal_into_its_union_carrier() {
    let session = TestSession::single(
        r#"
type Selector =
    | {
          kind: "value";
          value: int32;
      }
    | {
          kind: "flag";
          flag: boolean;
      };

function value(chosen: int32): Selector {
    { kind: "value", value: chosen }
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
@copy
type Selector = variant<uint8> { 0uint8 = ref<{ kind: void, value: int32 }, managed, mutable>; 1uint8 = ref<{ kind: void, flag: boolean }, managed, mutable>; };

function test.main.value(v0: int32): Selector {
entry(v0: int32):
    v1: void = undefined
    v2: { kind: void, value: int32 } = aggregate (v1, v0)
    v3: ref<{ kind: void, value: int32 }, managed, mutable> = new.complete v2
    v4: variant<uint8> { 0uint8 = ref<{ kind: void, value: int32 }, managed, mutable>; 1uint8 = ref<{ kind: void, flag: boolean }, managed, mutable>; } = variant.new 0, v3
    return v4
}
/// @layout.variant name=Selector size=16 align=8
/// @layout.discriminant owner=Selector kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=Selector index=0 discriminant=0 payload_offset=8
/// @layout.case owner=Selector index=1 discriminant=1 payload_offset=8
/// @layout.struct name=type@5 size=4 align=4
/// @layout.field owner=type@5 index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=type@5 index=1 name=value offset=0 size=4 align=4
/// @layout.struct name=type@9 size=1 align=1
/// @layout.field owner=type@9 index=0 name=kind offset=0 size=0 align=1
/// @layout.field owner=type@9 index=1 name=flag offset=0 size=1 align=1
/// @layout.variant name=type@12 size=16 align=8
/// @layout.discriminant owner=type@12 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@12 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@12 index=1 discriminant=1 payload_offset=8
"#);
}

#[test]
fn test_lower_a_nullish_union_alias_field_read() {
    let session = TestSession::single(
        r#"
type Label = string | undefined;

struct Meter {
    label: Label;
}

function read(meter: &readonly Meter): Label {
    return meter.label;
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

@copy
type Meter {
    label: ref<destack.string.string.String, managed, mutable, undefined>;
}

type Label = ref<destack.string.string.String, managed, mutable, undefined>;

function test.main.read<'a>(v0: ref<Meter, borrowed, 'a, readonly>): Label {
entry(v0: ref<Meter, borrowed, 'a, readonly>):
    v1: ref<Label, borrowed, readonly> = field.address v0, 0
    v2: Label = load v1
    return v2
}
/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=bytes offset=0 size=16 align=8
/// @layout.struct name=Meter size=8 align=8
/// @layout.field owner=Meter index=0 name=label offset=0 size=8 align=8
"#,
    );
}
