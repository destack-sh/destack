use crate::tests::TestSession;

/// Coalescing evaluates the fallback when both operands permit undefined.
#[test]
fn test_coalesce_optional_values() {
    let session = TestSession::single(
        r#"
declare function fallback(): int32 | undefined;

function choose(value: int32 | undefined): int32 | undefined {
    return value ?? fallback();
}
"#,
    );

    session.assert_mir_lowered("main.tspp", r#"
function test.main.choose(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): variant<uint1> { 0uint1 = int32; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    local l1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, readonly

entry(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l0
    variant.switch v1, 1 => b2, else b1

b1:
    store l1, v1
    jump b3

b2:
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = call test.main.fallback(): () => variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    store l1, v2
    jump b3

b3:
    v3: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l1
    return v3
}

external function test.main.fallback(): variant<uint1> { 0uint1 = int32; 1uint1 = void; }

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_three_union_arms_with_two_discriminant_bits() {
    let session = TestSession::single(
        r#"
function keep(value: int32 | boolean | float64): int32 | boolean | float64 {
    return value;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.keep", r#"
function test.main.keep(v0: variant<uint2> { 0uint2 = int32; 1uint2 = boolean; 2uint2 = float64; }): variant<uint2> { 0uint2 = int32; 1uint2 = boolean; 2uint2 = float64; } {
    local l0: variant<uint2> { 0uint2 = int32; 1uint2 = boolean; 2uint2 = float64; }

entry(v0: variant<uint2> { 0uint2 = int32; 1uint2 = boolean; 2uint2 = float64; }):
    store l0, v0
    v1: variant<uint2> { 0uint2 = int32; 1uint2 = boolean; 2uint2 = float64; } = load l0
    return v1
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.keep",
        r#"
type test.main.Shape = newtype<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }>;

function test.main.keep(v0: test.main.Shape): test.main.Shape {
    local l0: test.main.Shape

entry(v0: test.main.Shape):
    store l0, v0
    v1: test.main.Shape = load l0
    return v1
}
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

    session.assert_mir_function("main.tspp", "test.main.keep", r#"
function test.main.keep(v0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }): variant<uint1> { 0uint1 = boolean; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = boolean; 1uint1 = void; } = load l0
    return v1
}

/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.tspp", "test.main.forget", r#"
function test.main.forget(): variant<uint1> { 0uint1 = boolean; 1uint1 = void; } {
entry:
    v0: void = zeroed
    v1: variant<uint1> { 0uint1 = boolean; 1uint1 = void; } = variant.new 1
    return v1
}

/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#);
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.make",
        r#"
type test.main.Circle {
    kind: literal.string.circle;
    radius: float64;
}

type literal.string.circle { }

type test.main.Square {
    kind: literal.string.square;
    side: int32;
}

type test.main.Shape = newtype<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }>;

function test.main.make(v0: float64): test.main.Shape {
    local l0: float64

entry(v0: float64):
    store l0, v0
    v1: literal.string.circle = zeroed
    v2: float64 = load l0
    v3: test.main.Circle = aggregate (v1, v2)
    v4: variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; } = variant.new 0, v3
    v5: test.main.Shape = aggregate (v4)
    return v5
}

/// @layout.struct name=test.main.Circle size=8 align=8
/// @layout.field owner=test.main.Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=literal.string.circle size=0 align=1
/// @layout.struct name=test.main.Square size=4 align=4
/// @layout.field owner=test.main.Square index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Square index=1 name=side offset=0 size=4 align=4
/// @layout.variant name=type@11 size=16 align=8
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=8
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.pending",
        r#"
type test.main.Ready {
    state: literal.string.ready;
}

type test.main.Pending {
    state: literal.string.pending;
}

type literal.string.pending { }

type test.main.Status = newtype<variant<uint1> { 0uint1 = test.main.Ready; 1uint1 = test.main.Pending; }>;

function test.main.pending(): test.main.Status {
entry:
    v0: literal.string.pending = zeroed
    v1: test.main.Pending = aggregate (v0)
    v2: variant<uint1> { 0uint1 = test.main.Ready; 1uint1 = test.main.Pending; } = variant.new 1, v1
    v3: test.main.Status = aggregate (v2)
    return v3
}

/// @layout.struct name=test.main.Ready size=0 align=1
/// @layout.field owner=test.main.Ready index=0 name=state offset=0 size=0 align=1
/// @layout.struct name=test.main.Pending size=0 align=1
/// @layout.field owner=test.main.Pending index=0 name=state offset=0 size=0 align=1
/// @layout.struct name=literal.string.pending size=0 align=1
/// @layout.variant name=type@9 size=1 align=1
/// @layout.discriminant owner=type@9 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@9 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@9 index=1 discriminant=1 payload_offset=1
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

    session.assert_mir_function("main.tspp", "test.main.value", r#"
type literal.string.value { }

type literal.string.flag { }

function test.main.value(v0: int32): variant<uint1> { 0uint1 = ref<{ kind: literal.string.value, value: int32 }, managed, mutable, local>; 1uint1 = ref<{ kind: literal.string.flag, flag: boolean }, managed, mutable, local>; } {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: literal.string.value = zeroed
    v2: int32 = load l0
    v3: { kind: literal.string.value, value: int32 } = aggregate (v1, v2)
    v4: ref<{ kind: literal.string.value, value: int32 }, managed, mutable, local> = new.complete v3
    v5: variant<uint1> { 0uint1 = ref<{ kind: literal.string.value, value: int32 }, managed, mutable, local>; 1uint1 = ref<{ kind: literal.string.flag, flag: boolean }, managed, mutable, local>; } = variant.new 0, v4
    return v5
}

/// @layout.struct name=literal.string.value size=0 align=1
/// @layout.struct name=literal.string.flag size=0 align=1
/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=type@3 index=1 name=value offset=0 size=4 align=4
/// @layout.struct name=type@7 size=1 align=1
/// @layout.field owner=type@7 index=0 name=kind offset=0 size=0 align=1
/// @layout.field owner=type@7 index=1 name=flag offset=0 size=1 align=1
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=8
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

    session.assert_mir_function("main.tspp", "test.main.Listener.constructor", r#"
@nocopy
type test.main.Listener {
    value: int32;
}

constructor test.main.Listener.constructor<'a>(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.keep", r#"
@nocopy
type test.main.Listener {
    value: int32;
}

function test.main.keep(v0: ref<test.main.Listener, managed, mutable, local>): variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; } {
    local l0: ref<test.main.Listener, managed, mutable, local>
    local l1: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }

entry(v0: ref<test.main.Listener, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Listener, managed, mutable, local> = load l0
    v2: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v1
    store l1, v2
    v3: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; } = load l1
    return v3
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#);
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

    session.assert_mir_function("main.tspp", "test.main.Listener.constructor", r#"
@nocopy
type test.main.Listener {
    value: int32;
}

constructor test.main.Listener.constructor<'a>(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.accept",
        r#"
@nocopy
type test.main.Listener {
    value: int32;
}

function test.main.accept(v0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }): boolean {
    local l0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }):
    store l0, v0
    v1: uint1 = variant.tag.load l0
    v2: uint1 = 1
    v3: boolean = eq v1, v2
    v4: boolean = not v3
    return v4
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.forward",
        r#"
@nocopy
type test.main.Listener {
    value: int32;
}

function test.main.forward(v0: ref<test.main.Listener, managed, mutable, local>): boolean {
    local l0: ref<test.main.Listener, managed, mutable, local>

entry(v0: ref<test.main.Listener, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Listener, managed, mutable, local> = load l0
    v2: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v1
    v3: boolean = call test.main.accept(v2): (variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }) => boolean
    return v3
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
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

    session.assert_mir_function("main.tspp", "test.main.Listener.constructor", r#"
@nocopy
type test.main.Listener {
    value: int32;
}

constructor test.main.Listener.constructor<'a>(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.keep", r#"
@nocopy
type test.main.Listener {
    value: int32;
}

function test.main.keep(v0: ref<test.main.Listener, managed, mutable, local>): variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; } {
    local l0: ref<test.main.Listener, managed, mutable, local>

entry(v0: ref<test.main.Listener, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Listener, managed, mutable, local> = load l0
    v2: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v1
    return v2
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#);
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

    session.assert_mir_function("main.tspp", "test.main.Listener.constructor", r#"
@nocopy
type test.main.Listener {
    value: int32;
}

constructor test.main.Listener.constructor<'a>(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
@nocopy
type test.main.Listener {
    value: int32;
}

function test.main.read(v0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }): int32 {
    local l0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }):
    store l0, v0
    v1: uint1 = variant.tag.load l0
    v2: uint1 = 1
    v3: boolean = eq v1, v2
    v4: boolean = not v3
    branch v4 => b1 | b2

b1:
    v5: ref<test.main.Listener, borrowed, 'managed, mutable> = address (*(l0 as 0))
    v6: int32 = load (*v5).0
    return v6

b2:
    v7: int32 = 0
    return v7
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
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

    session.assert_mir_function("main.tspp", "test.main.keep", r#"
function test.main.keep(v0: boolean): variant<uint1> { 0uint1 = boolean; 1uint1 = void; } {
    local l0: boolean
    local l1: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }

entry(v0: boolean):
    store l0, v0
    v1: boolean = load l0
    v2: variant<uint1> { 0uint1 = boolean; 1uint1 = void; } = variant.new 0, v1
    store l1, v2
    v3: variant<uint1> { 0uint1 = boolean; 1uint1 = void; } = load l1
    return v3
}

/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#);
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.Circle.draw",
        r#"
@nocopy
type test.main.Circle { }

function test.main.Circle.draw(v0: ref<test.main.Circle, managed, mutable, local>): void {
    local l0: ref<test.main.Circle, managed, mutable, local>

entry(v0: ref<test.main.Circle, managed, mutable, local>):
    store l0, v0
    return
}

/// @layout.struct name=test.main.Circle size=0 align=1
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.keep", r#"
@nocopy
type test.main.Circle { }

@nocopy
type test.main.Drawable { }

function test.main.keep(v0: ref<test.main.Circle, managed, mutable, local>): variant<uint1> { 0uint1 = dynamic<test.main.Drawable, managed, mutable, local>; 1uint1 = void; } {
    local l0: ref<test.main.Circle, managed, mutable, local>
    local l1: variant<uint1> { 0uint1 = dynamic<test.main.Drawable, managed, mutable, local>; 1uint1 = void; }

entry(v0: ref<test.main.Circle, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Circle, managed, mutable, local> = load l0
    v2: dynamic<test.main.Drawable, managed, mutable, local> = dynamic.bind v1, ref<test.main.Circle, managed, mutable, local>
    v3: variant<uint1> { 0uint1 = dynamic<test.main.Drawable, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v2
    store l1, v3
    v4: variant<uint1> { 0uint1 = dynamic<test.main.Drawable, managed, mutable, local>; 1uint1 = void; } = load l1
    return v4
}

/// @layout.struct name=test.main.Circle size=0 align=1
/// @layout.struct name=test.main.Drawable size=0 align=1
/// @layout.variant name=type@8 size=16 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#);
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

    session.assert_mir_function("main.tspp", "test.main.Listener.constructor", r#"
@nocopy
type test.main.Listener {
    value: int32;
}

constructor test.main.Listener.constructor<'a>(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Listener>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.isHead", r#"
@nocopy
type test.main.Listener {
    value: int32;
}

function test.main.isHead(v0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }, v1: ref<test.main.Listener, managed, mutable, local>): boolean {
    local l0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }
    local l1: ref<test.main.Listener, managed, mutable, local>
    local l2: boolean, readonly

entry(v0: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; }, v1: ref<test.main.Listener, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; } = load l0
    v3: ref<test.main.Listener, managed, mutable, local> = load l1
    v4: variant<uint1> { 0uint1 = ref<test.main.Listener, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v3
    v5: uint1 = variant.tag v2
    v6: uint1 = variant.tag v4
    v7: boolean = eq v5, v6
    branch v7 => b1 | b2

b1:
    variant.switch v2, 0 => b4, 1 => b5, else b2

b2:
    v12: boolean = false
    store l2, v12
    jump b3

b3:
    v13: boolean = load l2
    v14: boolean = not v13
    return v14

b4:
    v8: ref<test.main.Listener, managed, mutable, local> = variant.payload v2, 0
    v9: ref<test.main.Listener, managed, mutable, local> = variant.payload v4, 0
    v10: boolean = eq v8, v9
    store l2, v10
    jump b3

b5:
    v11: boolean = true
    store l2, v11
    jump b3
}

/// @layout.struct name=test.main.Listener size=4 align=4
/// @layout.field owner=test.main.Listener index=0 name=value offset=0 size=4 align=4
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
type test.main.Meter {
    label: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

function test.main.read<'a>(v0: ref<test.main.Meter, borrowed, 'a, readonly>): variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } {
    local l0: ref<test.main.Meter, borrowed, 'a, readonly>

entry(v0: ref<test.main.Meter, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Meter, borrowed, 'a, readonly> = load l0
    v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load (*v1).0
    return v2
}

/// @layout.struct name=test.main.Meter size=8 align=8
/// @layout.field owner=test.main.Meter index=0 name=label offset=0 size=8 align=8
/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.circle",
        r#"
type literal.string.circle { }

function test.main.circle(): literal.string.circle {
entry:
    v0: literal.string.circle = zeroed
    return v0
}

/// @layout.struct name=literal.string.circle size=0 align=1
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.key",
        r#"
type literal.string.kind { }

function test.main.key(): literal.string.kind {
entry:
    v0: literal.string.kind = zeroed
    return v0
}

/// @layout.struct name=literal.string.kind size=0 align=1
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.kind",
        r#"
type test.main.Circle {
    kind: literal.string.circle;
    radius: float64;
}

type test.main.Square {
    kind: literal.string.square;
    side: float64;
}

type test.main.Shape = newtype<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }>;

@nocopy
@languageItem("string.String")
type String;

function test.main.kind(v0: test.main.Shape): ref<String, managed, mutable, local> {
    local l0: test.main.Shape
    local l1: ref<String, managed, mutable, local>, readonly

entry(v0: test.main.Shape):
    store l0, v0
    v1: test.main.Shape = load l0
    v2: variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; } = field.get v1, 0
    v3: uint1 = variant.tag v2
    switch v3, b2, 0 => b3, 1 => b4

b1:
    v6: ref<String, managed, mutable, local> = load l1
    return v6

b2:
    unreachable

b3:
    v4: ref<String, managed, mutable, local> = address @string.0
    store l1, v4
    jump b1

b4:
    v5: ref<String, managed, mutable, local> = address @string.1
    store l1, v5
    jump b1
}

/// @layout.struct name=test.main.Circle size=8 align=8
/// @layout.field owner=test.main.Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=test.main.Square size=8 align=8
/// @layout.field owner=test.main.Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Square index=1 name=side offset=0 size=8 align=8
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=8
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.computedKind",
        r#"
type test.main.Circle {
    kind: literal.string.circle;
    radius: float64;
}

type test.main.Square {
    kind: literal.string.square;
    side: float64;
}

type test.main.Shape = newtype<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }>;

type literal.string.kind { }

@nocopy
@languageItem("string.String")
type String;

function test.main.computedKind(v0: test.main.Shape): ref<String, managed, mutable, local> {
    local l0: test.main.Shape
    local l1: ref<String, managed, mutable, local>, readonly

entry(v0: test.main.Shape):
    store l0, v0
    v1: test.main.Shape = load l0
    v2: variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; } = field.get v1, 0
    v3: literal.string.kind = call test.main.key(): () => literal.string.kind
    v4: uint1 = variant.tag v2
    switch v4, b2, 0 => b3, 1 => b4

b1:
    v7: ref<String, managed, mutable, local> = load l1
    return v7

b2:
    unreachable

b3:
    v5: ref<String, managed, mutable, local> = address @string.0
    store l1, v5
    jump b1

b4:
    v6: ref<String, managed, mutable, local> = address @string.1
    store l1, v6
    jump b1
}

/// @layout.struct name=test.main.Circle size=8 align=8
/// @layout.field owner=test.main.Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=test.main.Square size=8 align=8
/// @layout.field owner=test.main.Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Square index=1 name=side offset=0 size=8 align=8
/// @layout.struct name=literal.string.kind size=0 align=1
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=8
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.label",
        r#"
type test.main.Circle {
    kind: literal.string.circle;
    radius: float64;
}

type test.main.Square {
    kind: literal.string.square;
    side: float64;
}

type test.main.Shape = newtype<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }>;

@nocopy
@languageItem("string.String")
type String;

function test.main.label(v0: test.main.Shape): ref<String, managed, mutable, local> {
    local l0: test.main.Shape
    local l1: ref<String, managed, mutable, local>, readonly

entry(v0: test.main.Shape):
    store l0, v0
    v1: test.main.Shape = load l0
    v2: variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; } = field.get v1, 0
    v3: uint1 = variant.tag v2
    switch v3, b2, 0 => b3, 1 => b4

b1:
    v6: ref<String, managed, mutable, local> = load l1
    return v6

b2:
    unreachable

b3:
    v4: ref<String, managed, mutable, local> = address @string.0
    store l1, v4
    jump b1

b4:
    v5: ref<String, managed, mutable, local> = address @string.1
    store l1, v5
    jump b1
}

/// @layout.struct name=test.main.Circle size=8 align=8
/// @layout.field owner=test.main.Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=test.main.Square size=8 align=8
/// @layout.field owner=test.main.Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Square index=1 name=side offset=0 size=8 align=8
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=8
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.matches",
        r#"
type test.main.Circle {
    kind: literal.string.circle;
    radius: float64;
}

type literal.string.circle { }

type test.main.Square {
    kind: literal.string.square;
    side: float64;
}

type test.main.Shape = newtype<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }>;

function test.main.matches(v0: test.main.Shape): boolean {
    local l0: test.main.Shape

entry(v0: test.main.Shape):
    store l0, v0
    v1: test.main.Shape = load l0
    v2: variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; } = field.get v1, 0
    v3: uint1 = variant.tag v2
    v4: uint1 = 0
    v5: boolean = eq v3, v4
    v6: literal.string.circle = call test.main.circle(): () => literal.string.circle
    v7: literal.string.circle = zeroed
    return v5
}

/// @layout.struct name=test.main.Circle size=8 align=8
/// @layout.field owner=test.main.Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=literal.string.circle size=0 align=1
/// @layout.struct name=test.main.Square size=8 align=8
/// @layout.field owner=test.main.Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Square index=1 name=side offset=0 size=8 align=8
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=8
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.differs",
        r#"
type test.main.Circle {
    kind: literal.string.circle;
    radius: float64;
}

type literal.string.circle { }

type test.main.Square {
    kind: literal.string.square;
    side: float64;
}

type test.main.Shape = newtype<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }>;

@nocopy
@languageItem("string.String")
type String;

function test.main.differs(v0: test.main.Shape): boolean {
    local l0: test.main.Shape

entry(v0: test.main.Shape):
    store l0, v0
    v1: literal.string.circle = call test.main.circle(): () => literal.string.circle
    v2: ref<String, managed, mutable, local> = address @string.0
    v3: test.main.Shape = load l0
    v4: variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; } = field.get v3, 0
    v5: uint1 = variant.tag v4
    v6: uint1 = 0
    v7: boolean = eq v5, v6
    v8: boolean = not v7
    return v8
}

/// @layout.struct name=test.main.Circle size=8 align=8
/// @layout.field owner=test.main.Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=literal.string.circle size=0 align=1
/// @layout.struct name=test.main.Square size=8 align=8
/// @layout.field owner=test.main.Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Square index=1 name=side offset=0 size=8 align=8
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=8
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

    session.assert_mir_function("main.tspp", "test.main.radius", r#"
type test.main.Circle {
    kind: literal.string.circle;
    radius: float64;
}

type test.main.Square {
    kind: literal.string.square;
    side: float64;
}

type test.main.Shape = newtype<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }>;

@nocopy
@languageItem("string.String")
type String;

function test.main.radius<'a>(v0: ref<test.main.Shape, borrowed, 'a, readonly>): float64 {
    local l0: ref<test.main.Shape, borrowed, 'a, readonly>

entry(v0: ref<test.main.Shape, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Shape, borrowed, 'a, readonly> = load l0
    v2: ref<variant<uint1> { 0uint1 = test.main.Circle; 1uint1 = test.main.Square; }, borrowed, 'a, readonly> = address (*v1).0
    v3: uint1 = variant.tag.load (*v2)
    v4: uint1 = 0
    v5: boolean = eq v3, v4
    v6: ref<String, managed, mutable, local> = address @string.0
    v7: boolean = not v5
    branch v7 => b1 | b2

b1:
    v8: float64 = 0
    return v8

b2:
    v9: ref<test.main.Shape, borrowed, 'a, readonly> = load l0
    v10: float64 = load ((*v9).0 as 0).1
    return v10
}

/// @layout.struct name=test.main.Circle size=8 align=8
/// @layout.field owner=test.main.Circle index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Circle index=1 name=radius offset=0 size=8 align=8
/// @layout.struct name=test.main.Square size=8 align=8
/// @layout.field owner=test.main.Square index=0 name=kind offset=8 size=0 align=1
/// @layout.field owner=test.main.Square index=1 name=side offset=0 size=8 align=8
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=8
"#);
}

#[test]
fn test_lower_a_match_through_nested_generic_alias_unions() {
    let session = TestSession::single(
        r#"
struct Ready<T: Copy> {
    kind: "ready" = "ready";
    value: T;
}

struct Waiting {
    kind: "waiting" = "waiting";
}

type Inner<T: Copy> = Ready<T> | Waiting;

type Outer<T: Copy> = Inner<T> | undefined;

function read(state: Outer<int32>): int32 {
    match (state) {
        Ready { value } => value
        _ => -1
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
type test.main.Waiting {
    kind: literal.string.waiting;
}

type test.main.Ready<T: Copy> {
    kind: literal.string.ready;
    value: T;
}

function test.main.read(v0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }): int32 {
    local l0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }
    local l1: int32
    local l2: int32

entry(v0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }):
    store l0, v0
    v1: uint2 = variant.tag.load l0
    switch v1, b2, 0 => b1

b1:
    v2: int32 = load (l0 as 0).1
    store l2, v2
    v3: int32 = load l2
    store l1, v3
    jump b4

b2:
    v4: int32 = -1
    store l1, v4
    jump b4

b3:
    unreachable

b4:
    v5: int32 = load l1
    return v5
}

/// @layout.struct name=test.main.Waiting size=0 align=1
/// @layout.field owner=test.main.Waiting index=0 name=kind offset=0 size=0 align=1
/// @layout.struct name=test.main.Ready<int32> size=4 align=4
/// @layout.field owner=test.main.Ready<int32> index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Ready<int32> index=1 name=value offset=0 size=4 align=4
/// @layout.variant name=type@19 size=8 align=4
/// @layout.discriminant owner=type@19 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@19 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@19 index=1 discriminant=1 payload_offset=4
/// @layout.case owner=type@19 index=2 discriminant=2 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_a_chain_field_read_through_an_optional_struct() {
    let session = TestSession::single(
        r#"
struct Context {
    id: int32;
}

struct Options {
    parent: Context | undefined;
}

function pick(options: Options | undefined): Context | undefined {
    return options?.parent;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.pick", r#"
type test.main.Context {
    id: int32;
}

type test.main.Options {
    parent: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; };
}

function test.main.pick(v0: variant<uint1> { 0uint1 = test.main.Options; 1uint1 = void; }): variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = test.main.Options; 1uint1 = void; }
    local l1: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = test.main.Options; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; } = load (l0 as 0).0
    store l1, v1
    jump b1

b1:
    v2: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; } = load l1
    return v2
}

/// @layout.struct name=test.main.Context size=4 align=4
/// @layout.field owner=test.main.Context index=0 name=id offset=0 size=4 align=4
/// @layout.struct name=test.main.Options size=8 align=4
/// @layout.field owner=test.main.Options index=0 name=parent offset=0 size=8 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@8 size=8 align=4
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#);
}

#[test]
fn test_lower_a_chain_read_of_an_optional_union_field() {
    let session = TestSession::single(
        r#"
struct Context {
    id: int32;
}

struct Options {
    parent?: Context | undefined;
}

function pick(options?: Options): Context | undefined {
    return options?.parent;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.pick", r#"
type test.main.Context {
    id: int32;
}

type test.main.Options {
    parent: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; };
}

function test.main.pick(v0: variant<uint1> { 0uint1 = test.main.Options; 1uint1 = void; }): variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = test.main.Options; 1uint1 = void; }
    local l1: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = test.main.Options; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; } = load (l0 as 0).0
    store l1, v1
    jump b1

b1:
    v2: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; } = load l1
    return v2
}

/// @layout.struct name=test.main.Context size=4 align=4
/// @layout.field owner=test.main.Context index=0 name=id offset=0 size=4 align=4
/// @layout.struct name=test.main.Options size=8 align=4
/// @layout.field owner=test.main.Options index=0 name=parent offset=0 size=8 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@8 size=8 align=4
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#);
}

#[test]
fn test_lower_a_plain_read_of_an_optional_union_field() {
    let session = TestSession::single(
        r#"
struct Context {
    id: int32;
}

struct Options {
    parent?: Context | undefined;
}

function pick(options: Options): Context | undefined {
    return options.parent;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pick",
        r#"
type test.main.Context {
    id: int32;
}

type test.main.Options {
    parent: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; };
}

function test.main.pick(v0: test.main.Options): variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; } {
    local l0: test.main.Options

entry(v0: test.main.Options):
    store l0, v0
    v1: variant<uint1> { 0uint1 = test.main.Context; 1uint1 = void; } = load (l0).0
    return v1
}

/// @layout.struct name=test.main.Context size=4 align=4
/// @layout.field owner=test.main.Context index=0 name=id offset=0 size=4 align=4
/// @layout.struct name=test.main.Options size=8 align=4
/// @layout.field owner=test.main.Options index=0 name=parent offset=0 size=8 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_a_borrowed_field_read_through_a_union_newtype_receiver() {
    let session = TestSession::single(
        r#"
newtype Left = {
    name: "Left";

    message: string;
};

newtype Right = {
    name: "Right";

    message: string;
};

newtype Either = Left | Right;

export extension of Either {
    text(): &readonly string {
        this.message
    }
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Either.text", r#"
type test.main.Left = newtype<{ name: literal.string.Left, message: ref<String, managed, mutable, local> }>;

type literal.string.Left { }

@nocopy
@languageItem("string.String")
type String;

type test.main.Right = newtype<{ name: literal.string.Right, message: ref<String, managed, mutable, local> }>;

type literal.string.Right { }

type test.main.Either = newtype<variant<uint1> { 0uint1 = ref<test.main.Left, managed, mutable, local>; 1uint1 = ref<test.main.Right, managed, mutable, local>; }>;

function test.main.Either.text<'a>(v0: ref<test.main.Either, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Either, borrowed, 'a, readonly>
    local l1: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly>, readonly

entry(v0: ref<test.main.Either, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Either, borrowed, 'a, readonly> = load l0
    v2: ref<variant<uint1> { 0uint1 = ref<test.main.Left, managed, mutable, local>; 1uint1 = ref<test.main.Right, managed, mutable, local>; }, borrowed, 'a, readonly> = address (*v1).0
    v3: uint1 = variant.tag.load (*v2)
    switch v3, b3, 0 => b1, 1 => b2

b1:
    v4: ref<ref<test.main.Left, managed, mutable, local>, borrowed, 'a, readonly> = address ((*v2) as 0)
    v5: ref<test.main.Left, managed, mutable, local> = load (*v4)
    v6: ref<test.main.Left, borrowed, 'managed, readonly> = cast.bit v5 -> ref<test.main.Left, borrowed, 'managed, readonly>
    v7: ref<test.main.Left, borrowed, 'a, readonly> = address (*v6)
    v8: ref<{ name: literal.string.Left, message: ref<String, managed, mutable, local> }, borrowed, 'a, readonly> = address (*v7).0
    v9: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v8).1
    store l1, v9
    jump b4

b2:
    v10: ref<ref<test.main.Right, managed, mutable, local>, borrowed, 'a, readonly> = address ((*v2) as 1)
    v11: ref<test.main.Right, managed, mutable, local> = load (*v10)
    v12: ref<test.main.Right, borrowed, 'managed, readonly> = cast.bit v11 -> ref<test.main.Right, borrowed, 'managed, readonly>
    v13: ref<test.main.Right, borrowed, 'a, readonly> = address (*v12)
    v14: ref<{ name: literal.string.Right, message: ref<String, managed, mutable, local> }, borrowed, 'a, readonly> = address (*v13).0
    v15: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v14).1
    store l1, v15
    jump b4

b3:
    unreachable

b4:
    v16: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = load l1
    v17: ref<String, managed, mutable, local> = load (*v16)
    v18: ref<String, borrowed, 'a, readonly> = cast.bit v17 -> ref<String, borrowed, 'a, readonly>
    return v18
}

/// @layout.struct name=literal.string.Left size=0 align=1
/// @layout.struct name=literal.string.Right size=0 align=1
/// @layout.struct name=type@9 size=8 align=8
/// @layout.field owner=type@9 index=0 name=name offset=8 size=0 align=1
/// @layout.field owner=type@9 index=1 name=message offset=0 size=8 align=8
/// @layout.struct name=type@14 size=8 align=8
/// @layout.field owner=type@14 index=0 name=name offset=8 size=0 align=1
/// @layout.field owner=type@14 index=1 name=message offset=0 size=8 align=8
/// @layout.variant name=type@18 size=16 align=8
/// @layout.discriminant owner=type@18 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@18 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@18 index=1 discriminant=1 payload_offset=8
"#);
}

#[test]
fn test_lower_a_match_through_a_borrowed_union_receiver() {
    let session = TestSession::single(
        r#"
struct Left {
    kind: "left" = "left";
    message?: string;
}

struct Right {
    kind: "right" = "right";
    message?: string;
}

newtype Either = Left | Right;

export extension of Either {
    text(&readonly this): &readonly string {
        match (this) {
            Left { message } => message ?? "left"
            Right { message } => message ?? "right"
        }
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Either.text",
        r#"
@nocopy
@languageItem("string.String")
type String;

type test.main.Either = newtype<variant<uint1> { 0uint1 = test.main.Left; 1uint1 = test.main.Right; }>;

function test.main.Either.text<'a>(v0: ref<test.main.Either, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Either, borrowed, 'a, readonly>
    local l1: ref<String, borrowed, 'a, readonly>
    local l2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }
    local l3: ref<String, borrowed, 'a, readonly>, readonly
    local l4: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }
    local l5: ref<String, borrowed, 'a, readonly>, readonly

entry(v0: ref<test.main.Either, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Either, borrowed, 'a, readonly> = load l0
    v2: uint1 = variant.tag.load (*v1).0
    switch v2, b3, 0 => b1, 1 => b2

b1:
    v3: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load ((*v1).0 as 0).1
    store l2, v3
    v4: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load l2
    variant.switch v4, 1 => b6, else b5

b2:
    v8: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load ((*v1).0 as 1).1
    store l4, v8
    v9: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load l4
    variant.switch v9, 1 => b9, else b8

b3:
    unreachable

b4:
    v13: ref<String, borrowed, 'a, readonly> = load l1
    return v13

b5:
    v5: ref<String, managed, mutable, local> = variant.payload v4, 0
    store l3, v5
    jump b7

b6:
    v6: ref<String, managed, mutable, local> = address @string.0
    store l3, v6
    jump b7

b7:
    v7: ref<String, borrowed, 'a, readonly> = load l3
    store l1, v7
    jump b4

b8:
    v10: ref<String, managed, mutable, local> = variant.payload v9, 0
    store l5, v10
    jump b10

b9:
    v11: ref<String, managed, mutable, local> = address @string.1
    store l5, v11
    jump b10

b10:
    v12: ref<String, borrowed, 'a, readonly> = load l5
    store l1, v12
    jump b4
}

/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
"#,
    );
    session.assert_mir_function("main.tspp", "test.main.Either.text", r#"
@nocopy
@languageItem("string.String")
type String;

type test.main.Either = newtype<variant<uint1> { 0uint1 = test.main.Left; 1uint1 = test.main.Right; }>;

function test.main.Either.text<'a>(v0: ref<test.main.Either, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Either, borrowed, 'a, readonly>
    local l1: ref<String, borrowed, 'a, readonly>
    local l2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }
    local l3: ref<String, borrowed, 'a, readonly>, readonly
    local l4: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }
    local l5: ref<String, borrowed, 'a, readonly>, readonly

entry(v0: ref<test.main.Either, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Either, borrowed, 'a, readonly> = load l0
    v2: uint1 = variant.tag.load (*v1).0
    switch v2, b3, 0 => b1, 1 => b2

b1:
    v3: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load ((*v1).0 as 0).1
    store l2, v3
    v4: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load l2
    variant.switch v4, 1 => b6, else b5

b2:
    v8: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load ((*v1).0 as 1).1
    store l4, v8
    v9: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load l4
    variant.switch v9, 1 => b9, else b8

b3:
    unreachable

b4:
    v13: ref<String, borrowed, 'a, readonly> = load l1
    return v13

b5:
    v5: ref<String, managed, mutable, local> = variant.payload v4, 0
    store l3, v5
    jump b7

b6:
    v6: ref<String, managed, mutable, local> = address @string.0
    store l3, v6
    jump b7

b7:
    v7: ref<String, borrowed, 'a, readonly> = load l3
    store l1, v7
    jump b4

b8:
    v10: ref<String, managed, mutable, local> = variant.payload v9, 0
    store l5, v10
    jump b10

b9:
    v11: ref<String, managed, mutable, local> = address @string.1
    store l5, v11
    jump b10

b10:
    v12: ref<String, borrowed, 'a, readonly> = load l5
    store l1, v12
    jump b4
}

/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
"#);
}

#[test]
fn test_lower_a_recursive_union_through_an_object_field() {
    let session = TestSession::single(
        r#"
type Selector = { kind: "all" } | { kind: "nested"; inner: Selector | undefined };

newtype Target = { kind: "any" } | { kind: "entity"; selector: Selector };

function keep(target: Target): Target {
    return target;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.keep",
        r#"
type test.main.Target = newtype<variant<uint1> { 0uint1 = ref<{ kind: literal.string.any }, managed, mutable, local>; 1uint1 = ref<{ kind: literal.string.entity, selector: variant<uint1> { 0uint1 = ref<{ kind: literal.string.all }, managed, mutable, local>; 1uint1 = ref<{ kind: literal.string.nested, inner: variant<uint2> { 0uint2 = ref<{ kind: literal.string.all }, managed, mutable, local>; 1uint2 = type@15; 2uint2 = void; } }, managed, mutable, local>; } }, managed, mutable, local>; }>;

function test.main.keep(v0: test.main.Target): test.main.Target {
    local l0: test.main.Target

entry(v0: test.main.Target):
    store l0, v0
    v1: test.main.Target = load l0
    return v1
}
"#,
    );
}

#[test]
fn test_coalesce_an_absent_recursive_newtype_case() {
    let session = TestSession::single(
        r#"
type JsonObject = { readonly [key: string]: Json };

type JsonArray = readonly Json[];

newtype Json =
    | { kind: "null" }
    | { kind: "boolean"; value: boolean }
    | { kind: "array"; value: JsonArray }
    | { kind: "object"; value: JsonObject };

function orElse(value: Json | undefined, fallback: Json): Json {
    return value ?? fallback;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.orElse",
        r#"
type test.main.Json = newtype<variant<uint2> { 0uint2 = ref<{ kind: literal.string.null }, managed, mutable, local>; 1uint2 = ref<{ kind: literal.string.boolean, value: boolean }, managed, mutable, local>; 2uint2 = ref<{ kind: literal.string.array, value: ref<Array<test.main.Json>, managed, readonly, local> }, managed, mutable, local>; 3uint2 = ref<{ kind: literal.string.object, value: dynamic<{  }, managed, mutable, local> }, managed, mutable, local>; }>;

function test.main.orElse(v0: variant<uint1> { 0uint1 = test.main.Json; 1uint1 = void; }, v1: test.main.Json): test.main.Json {
    local l0: variant<uint1> { 0uint1 = test.main.Json; 1uint1 = void; }
    local l1: test.main.Json
    local l2: test.main.Json, readonly

entry(v0: variant<uint1> { 0uint1 = test.main.Json; 1uint1 = void; }, v1: test.main.Json):
    store l0, v0
    store l1, v1
    v2: variant<uint1> { 0uint1 = test.main.Json; 1uint1 = void; } = load l0
    variant.switch v2, 1 => b2, else b1

b1:
    v3: test.main.Json = variant.payload v2, 0
    store l2, v3
    jump b3

b2:
    v4: test.main.Json = load l1
    store l2, v4
    jump b3

b3:
    v5: test.main.Json = load l2
    return v5
}

/// @layout.variant name=type@32 size=16 align=8
/// @layout.discriminant owner=type@32 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=4
/// @layout.case owner=type@32 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@32 index=1 discriminant=1 payload_offset=0
"#,
    );
}

/// Comparing an owned union with undefined reads its case at its place, moving nothing.
#[test]
fn test_compare_an_owned_union_with_undefined_without_moving_it() {
    let session = TestSession::single(
        r#"
struct Slot {
    value: ^Array<int32> | undefined;
}

function make(): ^Array<int32> | undefined {
    return undefined;
}

function hasStored(slot: &readonly Slot): boolean {
    return slot.value != undefined;
}

function hasLocal(): boolean {
    const value = make();
    return value != undefined;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.hasStored",
        r#"
type test.main.Slot {
    value: variant<uint1> { 0uint1 = Array<int32>; 1uint1 = void; };
}

function test.main.hasStored<'a>(v0: ref<test.main.Slot, borrowed, 'a, readonly>): boolean {
    local l0: ref<test.main.Slot, borrowed, 'a, readonly>

entry(v0: ref<test.main.Slot, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Slot, borrowed, 'a, readonly> = load l0
    v2: uint1 = variant.tag.load (*v1).0
    v3: uint1 = 1
    v4: boolean = eq v2, v3
    v5: boolean = not v4
    return v5
}

/// @layout.struct name=test.main.Slot size=32 align=8
/// @layout.field owner=test.main.Slot index=0 name=value offset=0 size=32 align=8
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.hasLocal", r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

function test.main.hasLocal(): boolean {
    local l0: variant<uint1> { 0uint1 = Array<int32>; 1uint1 = void; }

entry:
    v0: variant<uint1> { 0uint1 = Array<int32>; 1uint1 = void; } = call test.main.make(): () => variant<uint1> { 0uint1 = Array<int32>; 1uint1 = void; }
    store l0, v0
    v1: uint1 = variant.tag.load l0
    v2: uint1 = 1
    v3: boolean = eq v1, v2
    v4: boolean = not v3
    return v4
}

/// @layout.variant name=type@17 size=32 align=8
/// @layout.discriminant owner=type@17 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@17 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@17 index=1 discriminant=1 payload_offset=0
"#);
}

/// Narrow to a nested union alias by retagging every leaf the alias stands for.
#[test]
fn test_lower_a_narrowing_to_a_nested_generic_alias_union() {
    let session = TestSession::single(
        r#"
struct Ready<T: Copy> {
    kind: "ready" = "ready";
    value: T;
}

struct Waiting {
    kind: "waiting" = "waiting";
}

type Inner<T: Copy> = Ready<T> | Waiting;

type Outer<T: Copy> = Inner<T> | undefined;

function settle(state: Outer<int32>): Inner<int32> {
    if (state === undefined) {
        return Waiting {};
    }
    return state;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.settle",
        r#"
type test.main.Waiting {
    kind: literal.string.waiting;
}

type literal.string.waiting { }

type test.main.Ready<T: Copy> {
    kind: literal.string.ready;
    value: T;
}

function test.main.settle(v0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }): variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } {
    local l0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }
    local l1: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; }

entry(v0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }):
    store l0, v0
    v1: uint2 = variant.tag.load l0
    v2: uint2 = 2
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: literal.string.waiting = zeroed
    v5: test.main.Waiting = aggregate (v4)
    v6: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } = variant.new 1, v5
    return v6

b2:
    v7: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; } = load l0
    variant.switch v7, 0 => b5, 1 => b6, else b4

b3:
    v12: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } = load l1
    return v12

b4:
    panic

b5:
    v8: test.main.Ready<int32> = variant.payload v7, 0
    v9: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } = variant.new 0, v8
    store l1, v9
    jump b3

b6:
    v10: test.main.Waiting = variant.payload v7, 1
    v11: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } = variant.new 1, v10
    store l1, v11
    jump b3
}

/// @layout.struct name=test.main.Waiting size=0 align=1
/// @layout.field owner=test.main.Waiting index=0 name=kind offset=0 size=0 align=1
/// @layout.struct name=literal.string.waiting size=0 align=1
/// @layout.struct name=test.main.Ready<int32> size=4 align=4
/// @layout.field owner=test.main.Ready<int32> index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Ready<int32> index=1 name=value offset=0 size=4 align=4
/// @layout.variant name=type@19 size=8 align=4
/// @layout.discriminant owner=type@19 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@19 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@19 index=1 discriminant=1 payload_offset=4
/// @layout.case owner=type@19 index=2 discriminant=2 payload_offset=4
/// @layout.variant name=type@21 size=8 align=4
/// @layout.discriminant owner=type@21 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@21 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@21 index=1 discriminant=1 payload_offset=4
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.settle",
        r#"
type test.main.Waiting {
    kind: literal.string.waiting;
}

type literal.string.waiting { }

type test.main.Ready<T: Copy> {
    kind: literal.string.ready;
    value: T;
}

function test.main.settle(v0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }): variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } {
    local l0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }
    local l1: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; }

entry(v0: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; }):
    store l0, v0
    v1: uint2 = variant.tag.load l0
    v2: uint2 = 2
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: literal.string.waiting = zeroed
    v5: test.main.Waiting = aggregate (v4)
    v6: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } = variant.new 1, v5
    return v6

b2:
    v7: variant<uint2> { 0uint2 = test.main.Ready<int32>; 1uint2 = test.main.Waiting; 2uint2 = void; } = load l0
    variant.switch v7, 0 => b5, 1 => b6, else b4

b3:
    v12: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } = load l1
    return v12

b4:
    panic

b5:
    v8: test.main.Ready<int32> = variant.payload v7, 0
    v9: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } = variant.new 0, v8
    store l1, v9
    jump b3

b6:
    v10: test.main.Waiting = variant.payload v7, 1
    v11: variant<uint1> { 0uint1 = test.main.Ready<int32>; 1uint1 = test.main.Waiting; } = variant.new 1, v10
    store l1, v11
    jump b3
}

/// @layout.struct name=test.main.Waiting size=0 align=1
/// @layout.field owner=test.main.Waiting index=0 name=kind offset=0 size=0 align=1
/// @layout.struct name=literal.string.waiting size=0 align=1
/// @layout.struct name=test.main.Ready<int32> size=4 align=4
/// @layout.field owner=test.main.Ready<int32> index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Ready<int32> index=1 name=value offset=0 size=4 align=4
/// @layout.variant name=type@19 size=8 align=4
/// @layout.discriminant owner=type@19 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@19 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@19 index=1 discriminant=1 payload_offset=4
/// @layout.case owner=type@19 index=2 discriminant=2 payload_offset=4
/// @layout.variant name=type@21 size=8 align=4
/// @layout.discriminant owner=type@21 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@21 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@21 index=1 discriminant=1 payload_offset=4
"#,
    );
}
