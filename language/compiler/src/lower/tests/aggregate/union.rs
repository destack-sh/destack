use crate::tests::TestSession;

#[test]
fn test_lower_three_union_arms_with_two_discriminant_bits() {
    let session = TestSession::single(
        r#"
function keep(value: int32 | boolean | float64): int32 | boolean | float64 {
    return value;
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
function test.main.keep(v0: variant<uint2> { 0uint2 = int32; 1uint2 = boolean; 2uint2 = float64; }): variant<uint2> { 0uint2 = int32; 1uint2 = boolean; 2uint2 = float64; } {
entry(v0: variant<uint2> { 0uint2 = int32; 1uint2 = boolean; 2uint2 = float64; }):
    return v0
}
/// @layout.variant name=type@4 size=16 align=8
/// @layout.discriminant owner=type@4 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@4 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@4 index=1 discriminant=1 payload_offset=8
/// @layout.case owner=type@4 index=2 discriminant=2 payload_offset=8
"#);
}

#[test]
fn test_lower_discriminated_newtype_to_an_indexed_variant() {
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
type Shape = newtype<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }>;

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
/// @layout.variant name=type@12 size=16 align=8
/// @layout.discriminant owner=type@12 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@12 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@12 index=1 discriminant=1 payload_offset=8
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
function test.main.keep(v0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }): variant<uint1> { 0uint1 = boolean; 1uint1 = void; } {
entry(v0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }):
    return v0
}

function test.main.forget(): variant<uint1> { 0uint1 = boolean; 1uint1 = void; } {
entry:
    v0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; } = variant.new 1
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
fn test_lower_discriminated_newtype_construction_to_variant_new() {
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

newtype Shape = Circle | Square;

function make(radius: float64): Shape {
    return Shape(Circle { kind: "circle", radius });
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
type Shape = newtype<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }>;

function test.main.make(v0: float64): Shape {
entry(v0: float64):
    v1: Circle = aggregate (v0)
    v2: Shape = aggregate (v1)
    return v2
}
/// @layout.struct name=Circle size=8 align=8
/// @layout.field owner=Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=Square size=4 align=4
/// @layout.field owner=Square index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Square index=1 name=side offset=0 size=4 align=4
/// @layout.variant name=type@12 size=16 align=8
/// @layout.discriminant owner=type@12 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@12 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@12 index=1 discriminant=1 payload_offset=8
"#,
    );
}

#[test]
fn test_lower_singleton_union_arm_construction() {
    let session = TestSession::single(
        r#"
struct Ready {
    state: "ready";
}

struct Pending {
    state: "pending";
}

newtype Status = Ready | Pending;

function pending(): Status {
    Status(Pending { state: "pending" })
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
type Status = newtype<variant<uint1> { 0uint1 = Ready; 1uint1 = Pending; }>;

function test.main.pending(): Status {
entry:
    v0: Pending = aggregate ()
    v1: Status = aggregate (v0)
    return v1
}
/// @layout.struct name=Ready size=0 align=1
/// @layout.field owner=Ready index=0 name=state offset=0 size=0 align=1
/// @layout.struct name=Pending size=0 align=1
/// @layout.field owner=Pending index=0 name=state offset=0 size=0 align=1
/// @layout.variant name=type@8 size=1 align=1
/// @layout.discriminant owner=type@8 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=1
"#,
    );
}

#[test]
fn test_construct_an_object_literal_into_its_union_carrier() {
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
type Selector = variant<uint1> { 0uint1 = ref<{ kind: void, value: int32 }, managed, mutable>; 1uint1 = ref<{ kind: void, flag: boolean }, managed, mutable>; };

function test.main.value(v0: int32): Selector {
entry(v0: int32):
    v1: void = undefined
    v2: { kind: void, value: int32 } = aggregate (v1, v0)
    v3: ref<{ kind: void, value: int32 }, managed, mutable> = new.complete v2
    v4: variant<uint1> { 0uint1 = ref<{ kind: void, value: int32 }, managed, mutable>; 1uint1 = ref<{ kind: void, flag: boolean }, managed, mutable>; } = variant.new 0, v3
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

#[test]
fn test_lower_discriminant_reads_and_comparisons() {
    let session = TestSession::single(
        r#"
struct Circle {
    kind: "circle" = "circle";
    radius: float64;
}

struct Square {
    kind: "square" = "square";
    side: float64;
}

newtype Shape = Circle | Square;

function circle(): "circle" {
    return "circle";
}

function key(): "kind" {
    return "kind";
}

function kind(shape: Shape): "circle" | "square" {
    return shape.kind;
}

function computedKind(shape: Shape): "circle" | "square" {
    return shape[key()];
}

function label(shape: Shape): string {
    return shape.kind;
}

function matches(shape: Shape): boolean {
    return shape.kind == circle();
}

function differs(shape: Shape): boolean {
    return circle() !== shape.kind;
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
@copy
type Circle {
    kind: void;
    radius: float64;
}

@copy
type Square {
    kind: void;
    side: float64;
}

@copy
type Shape = newtype<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }>;

type destack.memory.unique.Unique<slice<uint8, managed, mutable>> = slice<uint8, unique, exclusive>;

type destack.string.string.String {
    bytes: destack.memory.unique.Unique<slice<uint8, managed, mutable>>;
}

immortal constant string.13298159783162365089.bytes: [uint8; 6] = b"circle"

immortal constant string.13298159783162365089: destack.string.string.String = {{globalAddress string.13298159783162365089.bytes, 6uint64}}

immortal constant string.11637857817615016681.bytes: [uint8; 6] = b"square"

immortal constant string.11637857817615016681: destack.string.string.String = {{globalAddress string.11637857817615016681.bytes, 6uint64}}

function test.main.circle(): void {
entry:
    v0: void = undefined
    return v0
}

function test.main.key(): void {
entry:
    v0: void = undefined
    return v0
}

function test.main.kind(v0: Shape): variant<uint1> { 0uint1 = void; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = void; }, readonly

entry(v0: Shape):
    v1: variant<uint1> { 0uint1 = Circle; 1uint1 = Square; } = field.get v0, 0
    v2: uint1 = variant.tag v1
    switch v2, b2, 0 => b3, 1 => b4

b1:
    v5: variant<uint1> { 0uint1 = void; 1uint1 = void; } = local.get l0
    return v5

b2:
    unreachable

b3:
    v3: variant<uint1> { 0uint1 = void; 1uint1 = void; } = variant.new 0
    local.set l0, v3
    jump b1

b4:
    v4: variant<uint1> { 0uint1 = void; 1uint1 = void; } = variant.new 1
    local.set l0, v4
    jump b1
}

function test.main.computedKind(v0: Shape): variant<uint1> { 0uint1 = void; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = void; }, readonly

entry(v0: Shape):
    v1: variant<uint1> { 0uint1 = Circle; 1uint1 = Square; } = field.get v0, 0
    call test.main.key(): () => void
    v2: void = undefined
    v3: uint1 = variant.tag v1
    switch v3, b2, 0 => b3, 1 => b4

b1:
    v6: variant<uint1> { 0uint1 = void; 1uint1 = void; } = local.get l0
    return v6

b2:
    unreachable

b3:
    v4: variant<uint1> { 0uint1 = void; 1uint1 = void; } = variant.new 0
    local.set l0, v4
    jump b1

b4:
    v5: variant<uint1> { 0uint1 = void; 1uint1 = void; } = variant.new 1
    local.set l0, v5
    jump b1
}

function test.main.label(v0: Shape): ref<destack.string.string.String, managed, mutable> {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = void; }, readonly
    local l1: ref<destack.string.string.String, managed, mutable>, readonly

entry(v0: Shape):
    v1: variant<uint1> { 0uint1 = Circle; 1uint1 = Square; } = field.get v0, 0
    v2: uint1 = variant.tag v1
    switch v2, b2, 0 => b3, 1 => b4

b1:
    v5: variant<uint1> { 0uint1 = void; 1uint1 = void; } = local.get l0
    variant.switch v5, 0 => b6, 1 => b7

b2:
    unreachable

b3:
    v3: variant<uint1> { 0uint1 = void; 1uint1 = void; } = variant.new 0
    local.set l0, v3
    jump b1

b4:
    v4: variant<uint1> { 0uint1 = void; 1uint1 = void; } = variant.new 1
    local.set l0, v4
    jump b1

b5:
    v8: ref<destack.string.string.String, managed, mutable> = local.get l1
    return v8

b6:
    v6: ref<destack.string.string.String, managed, mutable> = global.address string.13298159783162365089
    local.set l1, v6
    jump b5

b7:
    v7: ref<destack.string.string.String, managed, mutable> = global.address string.11637857817615016681
    local.set l1, v7
    jump b5
}

function test.main.matches(v0: Shape): boolean {
entry(v0: Shape):
    v1: variant<uint1> { 0uint1 = Circle; 1uint1 = Square; } = field.get v0, 0
    v2: uint1 = variant.tag v1
    v3: uint1 = 0
    v4: boolean = int.eq v2, v3
    call test.main.circle(): () => void
    v5: void = undefined
    return v4
}

function test.main.differs(v0: Shape): boolean {
entry(v0: Shape):
    call test.main.circle(): () => void
    v1: void = undefined
    v2: variant<uint1> { 0uint1 = void; 1uint1 = void; } = variant.new 0
    v3: variant<uint1> { 0uint1 = Circle; 1uint1 = Square; } = field.get v0, 0
    v4: uint1 = variant.tag v3
    v5: uint1 = 0
    v6: boolean = int.eq v4, v5
    v7: boolean = int.not v6
    return v7
}
/// @layout.struct name=Circle size=8 align=8
/// @layout.field owner=Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=Square size=8 align=8
/// @layout.field owner=Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Square index=1 name=side offset=0 size=8 align=8
/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=bytes offset=0 size=16 align=8
/// @layout.variant name=type@11 size=16 align=8
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@15 size=1 align=1
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=1
"#);
}

#[test]
fn test_lower_borrowed_narrowed_union_payload() {
    let session = TestSession::single(
        r#"
struct Circle {
    kind: "circle" = "circle";
    radius: float64;
}

struct Square {
    kind: "square" = "square";
    side: float64;
}

newtype Shape = Circle | Square;

function radius(shape: &readonly Shape): float64 {
    if (shape.kind !== "circle") {
        return 0.0;
    }

    return shape.radius;
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
@copy
type Circle {
    kind: void;
    radius: float64;
}

@copy
type Square {
    kind: void;
    side: float64;
}

@copy
type Shape = newtype<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }>;

type destack.memory.unique.Unique<slice<uint8, managed, mutable>> = slice<uint8, unique, exclusive>;

type destack.string.string.String {
    bytes: destack.memory.unique.Unique<slice<uint8, managed, mutable>>;
}

immortal constant string.13298159783162365089.bytes: [uint8; 6] = b"circle"

immortal constant string.13298159783162365089: destack.string.string.String = {{globalAddress string.13298159783162365089.bytes, 6uint64}}

function test.main.radius<'a>(v0: ref<Shape, borrowed, 'a, readonly>): float64 {
entry(v0: ref<Shape, borrowed, 'a, readonly>):
    v1: ref<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }, borrowed, 'a, readonly> = field.address v0, 0
    v2: uint1 = variant.tag.load v1
    v3: uint1 = 0
    v4: boolean = int.eq v2, v3
    v5: variant<uint2> { 0uint2 = void; 1uint2 = void; 2uint2 = void; } = variant.new 2
    v6: boolean = int.not v4
    branch v6 => b1 | b2

b1:
    v7: float64 = 0
    return v7

b2:
    v8: ref<float64, borrowed, readonly> = field.address v0, 1
    v9: float64 = load v8
    return v9
}
/// @layout.struct name=Circle size=8 align=8
/// @layout.field owner=Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=Square size=8 align=8
/// @layout.field owner=Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Square index=1 name=side offset=0 size=8 align=8
/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=bytes offset=0 size=16 align=8
/// @layout.variant name=type@11 size=16 align=8
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@35 size=1 align=1
/// @layout.discriminant owner=type@35 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@35 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@35 index=1 discriminant=1 payload_offset=1
/// @layout.case owner=type@35 index=2 discriminant=2 payload_offset=1
"#);
}
