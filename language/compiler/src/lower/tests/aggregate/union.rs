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
fn test_construct_an_object_literal_into_its_union_representation() {
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
type Selector = variant<uint1> { 0uint1 = ref<{ kind: void, value: int32 }, managed, mutable, local>; 1uint1 = ref<{ kind: void, flag: boolean }, managed, mutable, local>; };

function test.main.value(v0: int32): Selector {
entry(v0: int32):
    v1: void = undefined
    v2: { kind: void, value: int32 } = aggregate (v1, v0)
    v3: ref<{ kind: void, value: int32 }, managed, mutable, local> = new.complete v2
    v4: Selector = variant.new 0, v3
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
"#);
}

#[test]
fn test_inject_a_reference_into_its_nullish_union_store() {
    let session = TestSession::single(
        r#"
class Listener {
    value: int32 = 0;
}

function keep(listener: Listener): Listener | undefined {
    let head: Listener | undefined = listener;

    head
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type Listener {
    value: int32;
}

function test.main.Listener.constructor(v0: ref<uninit<Listener>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Listener>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.keep(v0: ref<Listener, managed, mutable, local>): ref<Listener, managed, mutable, undefined, local> {
    local l0: ref<Listener, managed, mutable, undefined, local>

entry(v0: ref<Listener, managed, mutable, local>):
    v1: ref<Listener, managed, mutable, undefined, local> = cast.bit v0 -> ref<Listener, managed, mutable, undefined, local>
    local.set l0, v1
    v2: ref<Listener, managed, mutable, undefined, local> = local.get l0
    return v2
}

/// @layout.struct name=Listener size=4 align=4
/// @layout.field owner=Listener index=0 name=value offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_inject_a_reference_argument_into_a_nullish_parameter() {
    let session = TestSession::single(
        r#"
class Listener {
    value: int32 = 0;
}

function accept(head: Listener | undefined): boolean {
    head !== undefined
}

function forward(listener: Listener): boolean {
    accept(listener)
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type Listener {
    value: int32;
}

function test.main.Listener.constructor(v0: ref<uninit<Listener>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Listener>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.accept(v0: ref<Listener, managed, mutable, undefined, local>): boolean {
entry(v0: ref<Listener, managed, mutable, undefined, local>):
    v1: ref<Listener, managed, mutable, undefined, local> = undefined
    v2: boolean = eq v0, v1
    v3: boolean = not v2
    return v3
}

function test.main.forward(v0: ref<Listener, managed, mutable, local>): boolean {
entry(v0: ref<Listener, managed, mutable, local>):
    v1: ref<Listener, managed, mutable, undefined, local> = cast.bit v0 -> ref<Listener, managed, mutable, undefined, local>
    v2: boolean = call test.main.accept(v1): (ref<Listener, managed, mutable, undefined, local>) => boolean
    return v2
}

/// @layout.struct name=Listener size=4 align=4
/// @layout.field owner=Listener index=0 name=value offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_inject_a_returned_reference_into_a_nullish_result() {
    let session = TestSession::single(
        r#"
class Listener {
    value: int32 = 0;
}

function keep(listener: Listener): Listener | undefined {
    listener
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type Listener {
    value: int32;
}

function test.main.Listener.constructor(v0: ref<uninit<Listener>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Listener>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.keep(v0: ref<Listener, managed, mutable, local>): ref<Listener, managed, mutable, undefined, local> {
entry(v0: ref<Listener, managed, mutable, local>):
    v1: ref<Listener, managed, mutable, undefined, local> = cast.bit v0 -> ref<Listener, managed, mutable, undefined, local>
    return v1
}

/// @layout.struct name=Listener size=4 align=4
/// @layout.field owner=Listener index=0 name=value offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_read_a_narrowed_reference_out_of_its_nullish_union() {
    let session = TestSession::single(
        r#"
class Listener {
    value: int32 = 0;
}

function read(head: Listener | undefined): int32 {
    if (head !== undefined) {
        return head.value;
    }

    0
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Listener {
    value: int32;
}

function test.main.Listener.constructor(v0: ref<uninit<Listener>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Listener>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.read(v0: ref<Listener, managed, mutable, undefined, local>): int32 {
entry(v0: ref<Listener, managed, mutable, undefined, local>):
    v1: ref<Listener, managed, mutable, undefined, local> = undefined
    v2: boolean = eq v0, v1
    v3: boolean = not v2
    branch v3 => b1 | b2

b1:
    v4: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v5: int32 = load v4
    return v5

b2:
    v6: int32 = 0
    return v6
}

/// @layout.struct name=Listener size=4 align=4
/// @layout.field owner=Listener index=0 name=value offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_inject_a_boolean_into_its_nullish_union_store() {
    let session = TestSession::single(
        r#"
function keep(flag: boolean): boolean | undefined {
    let stored: boolean | undefined = flag;

    stored
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
function test.main.keep(v0: boolean): variant<uint1> { 0uint1 = boolean; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }

entry(v0: boolean):
    v1: variant<uint1> { 0uint1 = boolean; 1uint1 = void; } = variant.new 0, v0
    local.set l0, v1
    v2: variant<uint1> { 0uint1 = boolean; 1uint1 = void; } = local.get l0
    return v2
}

/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#,
    );
}

#[test]
fn test_inject_an_erased_value_into_a_nullish_interface_union() {
    let session = TestSession::single(
        r#"
interface Drawable {
    draw(): void;
}

class Circle {
    draw(): void {}
}

function keep(circle: Circle): Drawable | undefined {
    let held: Drawable | undefined = circle;

    held
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type Circle { }

type Drawable { }

function test.main.Circle.draw(v0: ref<Circle, managed, mutable, local>): void {
entry(v0: ref<Circle, managed, mutable, local>):
    return
}

function test.main.keep(v0: ref<Circle, managed, mutable, local>): dynamic<Drawable, managed, mutable, undefined, local> {
    local l0: dynamic<Drawable, managed, mutable, undefined, local>

entry(v0: ref<Circle, managed, mutable, local>):
    v1: dynamic<Drawable, managed, mutable, local> = dynamic.bind v0, Circle
    v2: dynamic<Drawable, managed, mutable, undefined, local> = cast.bit v1 -> dynamic<Drawable, managed, mutable, undefined, local>
    local.set l0, v2
    v3: dynamic<Drawable, managed, mutable, undefined, local> = local.get l0
    return v3
}

/// @layout.struct name=Circle size=0 align=1
/// @layout.struct name=Drawable size=0 align=1

/// @dispatch.shape constraint=type@5 function=draw
"#,
    );
}

#[test]
fn test_widen_identity_comparison_over_a_nullish_reference() {
    let session = TestSession::single(
        r#"
class Listener {
    value: int32 = 0;
}

function isHead(head: Listener | undefined, listener: Listener): boolean {
    head !== listener
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type Listener {
    value: int32;
}

function test.main.Listener.constructor(v0: ref<uninit<Listener>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Listener>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.isHead(v0: ref<Listener, managed, mutable, undefined, local>, v1: ref<Listener, managed, mutable, local>): boolean {
entry(v0: ref<Listener, managed, mutable, undefined, local>, v1: ref<Listener, managed, mutable, local>):
    v2: ref<Listener, managed, mutable, undefined, local> = cast.bit v1 -> ref<Listener, managed, mutable, undefined, local>
    v3: boolean = eq v0, v2
    v4: boolean = not v3
    return v4
}

/// @layout.struct name=Listener size=4 align=4
/// @layout.field owner=Listener index=0 name=value offset=0 size=4 align=4
"#,
    );
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
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

type Label = ref<String, managed, mutable, undefined, local>;

@copy
type Meter {
    label: Label;
}

function test.main.read<'a>(v0: ref<Meter, borrowed, 'a, readonly, local>): Label {
entry(v0: ref<Meter, borrowed, 'a, readonly, local>):
    v1: ref<Label, borrowed, readonly, local> = field.address v0, 0
    v2: Label = load v1
    return v2
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
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
    side: float64;
}

@copy
type Shape = newtype<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }>;

@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

constant string.0: String = "circle"
constant string.1: String = "square"

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

function test.main.label(v0: Shape): ref<String, managed, mutable, local> {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = void; }, readonly
    local l1: ref<String, managed, mutable, local>, readonly

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
    v8: ref<String, managed, mutable, local> = local.get l1
    return v8

b6:
    v6: ref<String, managed, mutable, local> = global.address string.0
    local.set l1, v6
    jump b5

b7:
    v7: ref<String, managed, mutable, local> = global.address string.1
    local.set l1, v7
    jump b5
}

function test.main.matches(v0: Shape): boolean {
entry(v0: Shape):
    v1: variant<uint1> { 0uint1 = Circle; 1uint1 = Square; } = field.get v0, 0
    v2: uint1 = variant.tag v1
    v3: uint1 = 0
    v4: boolean = eq v2, v3
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
    v6: boolean = eq v4, v5
    v7: boolean = not v6
    return v7
}

/// @layout.struct name=Circle size=8 align=8
/// @layout.field owner=Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=Square size=8 align=8
/// @layout.field owner=Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Square index=1 name=side offset=0 size=8 align=8
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.variant name=type@11 size=16 align=8
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@15 size=1 align=1
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=1
"#,
    );
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
    side: float64;
}

@copy
type Shape = newtype<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }>;

@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

constant string.0: String = "circle"

function test.main.radius<'a>(v0: ref<Shape, borrowed, 'a, readonly, local>): float64 {
entry(v0: ref<Shape, borrowed, 'a, readonly, local>):
    v1: ref<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }, borrowed, 'a, readonly, local> = field.address v0, 0
    v2: uint1 = variant.tag.load v1
    v3: uint1 = 0
    v4: boolean = eq v2, v3
    v5: variant<uint2> { 0uint2 = void; 1uint2 = void; 2uint2 = void; } = variant.new 2
    v6: boolean = not v4
    branch v6 => b1 | b2

b1:
    v7: float64 = 0
    return v7

b2:
    v8: ref<variant<uint1> { 0uint1 = Circle; 1uint1 = Square; }, borrowed, 'a, readonly, local> = field.address v0, 0
    v9: ref<Circle, borrowed, 'a, readonly, local> = variant.payload.address v8, 0
    v10: ref<float64, borrowed, readonly, local> = field.address v9, 1
    v11: float64 = load v10
    return v11
}

/// @layout.struct name=Circle size=8 align=8
/// @layout.field owner=Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=Square size=8 align=8
/// @layout.field owner=Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=Square index=1 name=side offset=0 size=8 align=8
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.variant name=type@11 size=16 align=8
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@31 size=1 align=1
/// @layout.discriminant owner=type@31 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@31 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@31 index=1 discriminant=1 payload_offset=1
/// @layout.case owner=type@31 index=2 discriminant=2 payload_offset=1
"#,
    );
}
