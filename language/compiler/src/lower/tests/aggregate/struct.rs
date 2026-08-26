use crate::tests::TestSession;

/// A struct literal lowers to one aggregate over its field values.
#[test]
fn test_lower_struct_literal_to_aggregate() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function origin(): Point {
    return Point { x: 0, y: 0 };
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
    y: int32;
}

function test.main.origin(): Point {
entry:
    v0: int32 = 0
    v1: int32 = 0
    v2: Point = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

/// Reading a struct field lowers to a field get at the field's index.
#[test]
fn test_lower_struct_field_read() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function abscissa(point: Point): int32 {
    return point.x;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
    y: int32;
}

function test.main.abscissa(v0: Point): int32 {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    return v1
}

/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

/// A nested struct literal lowers to one aggregate per level.
#[test]
fn test_lower_nested_struct_literal_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

struct Segment {
    start: Point;
    end: Point;
}

function diagonal(size: int32): Segment {
    return Segment { start: Point { x: 0, y: 0 }, end: Point { x: size, y: size } };
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
    y: int32;
}

@copy
type Segment {
    start: Point;
    end: Point;
}

function test.main.diagonal(v0: int32): Segment {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 0
    v3: Point = aggregate (v1, v2)
    v4: Point = aggregate (v0, v0)
    v5: Segment = aggregate (v3, v4)
    return v5
}

/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
/// @layout.struct name=Segment size=16 align=4
/// @layout.field owner=Segment index=0 name=start offset=0 size=8 align=4
/// @layout.field owner=Segment index=1 name=end offset=8 size=8 align=4
"#,
    );
}

/// A lifetime generic struct keeps its written lifetime on the lowered slice field.
#[test]
fn test_lower_lifetime_generic_struct_literal_to_aggregate() {
    let session = TestSession::single(
        r#"
struct Entry<'a> {
    name: &'a [uint8];
}

function make(name: &[uint8]): void {
    const entry = Entry { name };
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Entry<'a> {
    name: slice<uint8, borrowed, 'a, mutable, local>;
}

function test.main.make<'a>(v0: slice<uint8, borrowed, 'a, mutable, local>): void {
entry(v0: slice<uint8, borrowed, 'a, mutable, local>):
    v1: Entry<'a> = aggregate (v0)
    return
}

/// @layout.struct name=Entry size=16 align=8
/// @layout.field owner=Entry index=0 name=name offset=0 size=16 align=8
"#,
    );
}

/// A class whose methods stay uncalled lowers its constructor alone.
#[test]
fn test_lower_this_typed_return_through_the_receiver_form() {
    let session = TestSession::single(
        r#"
class Counter {
    total: int32 = 0;

    read(&readonly this): int32 {
        return this.total;
    }

    double(&readonly this): int32 {
        return this.read() * 2;
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    total: int32;
}

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.Counter.read<'a>(v0: ref<Counter, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<Counter, borrowed, 'a, readonly, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function test.main.Counter.double<'a>(v0: ref<Counter, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<Counter, borrowed, 'a, readonly, local>):
    v1: int32 = call test.main.Counter.read(v0): <'a>(ref<Counter, borrowed, 'a, readonly, local>) => int32
    v2: int32 = 2
    v3: int32 = mul v1, v2
    return v3
}

/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=total offset=0 size=4 align=4
"#,
    );
}

/// An omitted optional field lowers to the variant's void case.
#[test]
fn test_store_an_absent_optional_struct_field_as_undefined() {
    let session = TestSession::single(
        r#"
struct Options {
    count: int32;
    limit?: int32;
}

function make(): Options {
    Options { count: 1 }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Options {
    count: int32;
    limit: variant<uint1> { 0uint1 = int32; 1uint1 = void; };
}

function test.main.make(): Options {
entry:
    v0: int32 = 1
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    v2: Options = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=Options size=12 align=4
/// @layout.field owner=Options index=0 name=count offset=0 size=4 align=4
/// @layout.field owner=Options index=1 name=limit offset=4 size=8 align=4
/// @layout.variant name=type@5 size=8 align=4
/// @layout.discriminant owner=type@5 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@5 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@5 index=1 discriminant=1 payload_offset=4
"#,
    );
}

/// An omitted field evaluates its declared initializer at the construction.
#[test]
fn test_lower_omitted_field_evaluates_its_initializer() {
    let session = TestSession::single(
        r#"
struct Counter {
    count: int32 = 3;
    label: int32;
}

function make(): Counter {
    return Counter { label: 7 };
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Counter {
    count: int32;
    label: int32;
}

function test.main.make(): Counter {
entry:
    v0: int32 = 3
    v1: int32 = 7
    v2: Counter = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=Counter size=8 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
/// @layout.field owner=Counter index=1 name=label offset=4 size=4 align=4
"#,
    );
}

/// Constructing an imported struct evaluates the initializers its own module declares.
#[test]
fn test_lower_imported_construction_evaluates_foreign_initializers() {
    let session = TestSession::builder()
        .module(
            "counter.ds",
            r#"
export struct Counter {
    count: int32 = 3;
    label: int32;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Counter } from "./counter";

function make(): Counter {
    return Counter { label: 7 };
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type test.counter.Counter {
    count: int32;
    label: int32;
}

function test.main.make(): test.counter.Counter {
entry:
    v0: int32 = 3
    v1: int32 = 7
    v2: test.counter.Counter = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=test.counter.Counter size=8 align=4
/// @layout.field owner=test.counter.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.field owner=test.counter.Counter index=1 name=label offset=4 size=4 align=4
"#,
    );
}

/// Constructing a generic struct grounds its initializer against the inferred arguments.
#[test]
fn test_lower_generic_construction_grounds_initializer_parameters() {
    let session = TestSession::single(
        r#"
struct Slot<T> {
    value: T;
    index: isize = 0;
}

function fill(): Slot<int32> {
    return Slot { value: 9 };
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Slot<int32> {
    value: int32;
    index: isize;
}

function test.main.fill(): Slot<int32> {
entry:
    v0: int32 = 9
    v1: isize = 0
    v2: Slot<int32> = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=Slot<int32> size=16 align=8
/// @layout.field owner=Slot<int32> index=0 name=value offset=8 size=4 align=4
/// @layout.field owner=Slot<int32> index=1 name=index offset=0 size=8 align=8
"#,
    );
}
