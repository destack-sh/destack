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

    session.assert_mir_function(
        "main.ds",
        "test.main.origin",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.origin(): test.main.Point {
entry:
    v0: int32 = 0
    v1: int32 = 0
    v2: test.main.Point = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.abscissa",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.abscissa(v0: test.main.Point): int32 {
    local l0: test.main.Point

entry(v0: test.main.Point):
    store l0, v0
    v1: int32 = load (l0).0
    return v1
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.diagonal",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

type test.main.Segment {
    start: test.main.Point;
    end: test.main.Point;
}

function test.main.diagonal(v0: int32): test.main.Segment {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = 0
    v2: int32 = 0
    v3: test.main.Point = aggregate (v1, v2)
    v4: int32 = load l0
    v5: int32 = load l0
    v6: test.main.Point = aggregate (v4, v5)
    v7: test.main.Segment = aggregate (v3, v6)
    return v7
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.struct name=test.main.Segment size=16 align=4
/// @layout.field owner=test.main.Segment index=0 name=start offset=0 size=8 align=4
/// @layout.field owner=test.main.Segment index=1 name=end offset=8 size=8 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.make",
        r#"
type test.main.Entry<'a> {
    name: slice<uint8, borrowed, 'a, mutable>;
}

function test.main.make<'a>(v0: slice<uint8, borrowed, 'a, mutable>): void {
    local l0: slice<uint8, borrowed, 'a, mutable>
    local l1: test.main.Entry<'a>

entry(v0: slice<uint8, borrowed, 'a, mutable>):
    store l0, v0
    v1: slice<uint8, borrowed, 'a, mutable> = load l0
    v2: test.main.Entry<'a> = aggregate (v1)
    store l1, v2
    return
}

/// @layout.struct name=test.main.Entry<'a> size=16 align=8
/// @layout.field owner=test.main.Entry<'a> index=0 name=name offset=0 size=16 align=8
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

    session.assert_mir_function(
        "main.ds",
        "test.main.Counter.constructor",
        r#"
@nocopy
type test.main.Counter {
    total: int32;
}

constructor test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=total offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Counter.read",
        r#"
@nocopy
type test.main.Counter {
    total: int32;
}

function test.main.Counter.read<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=total offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Counter.double",
        r#"
@nocopy
type test.main.Counter {
    total: int32;
}

function test.main.Counter.double<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v2: int32 = call test.main.Counter.read(v1): (ref<test.main.Counter, borrowed, 'a, readonly>) => int32
    v3: int32 = 2
    v4: int32 = mul v2, v3
    return v4
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=total offset=0 size=4 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.make",
        r#"
type test.main.Options {
    count: int32;
    limit: variant<uint1> { 0uint1 = int32; 1uint1 = void; };
}

function test.main.make(): test.main.Options {
entry:
    v0: int32 = 1
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    v2: test.main.Options = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=test.main.Options size=12 align=4
/// @layout.field owner=test.main.Options index=0 name=count offset=0 size=4 align=4
/// @layout.field owner=test.main.Options index=1 name=limit offset=4 size=8 align=4
/// @layout.variant name=type@4 size=8 align=4
/// @layout.discriminant owner=type@4 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@4 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@4 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.make",
        r#"
type test.main.Counter {
    count: int32;
    label: int32;
}

function test.main.make(): test.main.Counter {
entry:
    v0: int32 = 3
    v1: int32 = 7
    v2: test.main.Counter = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=test.main.Counter size=8 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.field owner=test.main.Counter index=1 name=label offset=4 size=4 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.make",
        r#"
type test.counter.Counter;

function test.main.make(): test.counter.Counter {
entry:
    v0: int32 = 3
    v1: int32 = 7
    v2: test.counter.Counter = aggregate (v0, v1)
    return v2
}
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

    session.assert_mir_function(
        "main.ds",
        "test.main.fill",
        r#"
type test.main.Slot<T> {
    value: T;
    index: isize;
}

function test.main.fill(): test.main.Slot<int32> {
entry:
    v0: int32 = 9
    v1: isize = 0
    v2: test.main.Slot<int32> = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=test.main.Slot<int32> size=16 align=8
/// @layout.field owner=test.main.Slot<int32> index=0 name=value offset=8 size=4 align=4
/// @layout.field owner=test.main.Slot<int32> index=1 name=index offset=0 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_a_conditional_field_borrow_into_a_struct_literal() {
    let session = TestSession::single(
        r#"
struct Entry<'a> {
    name: &'a readonly string;
    unit?: &'a readonly string | undefined;
}

function write(entry: &readonly Entry): void {}

class Counter {
    readonly name: string;
    readonly unit?: string;

    constructor(name: string, unit?: string) {
        this.name = name;
        this.unit = unit;
    }

    add(&readonly this): void {
        const entry = Entry {
            name: &readonly this.name,
            unit: this.unit == undefined ? undefined : &readonly this.unit,
        };
        write(&readonly entry);
    }
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Counter.add",
        r#"
@nocopy
type test.main.Counter {
    name: ref<String, managed, mutable, local>;
    unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

type test.main.Entry<'a> {
    name: ref<String, borrowed, 'a, readonly>;
    unit: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; };
}

function test.main.Counter.add<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly>): void {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly>
    local l1: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; }
    local l2: test.main.Entry<'a>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v1).0
    v3: ref<String, managed, mutable, local> = load (*v2)
    v4: ref<String, borrowed, 'a, readonly> = cast.bit v3 -> ref<String, borrowed, 'a, readonly>
    v5: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v6: uint1 = variant.tag.load (*v5).1
    v7: uint1 = 1
    v8: boolean = eq v6, v7
    branch v8 => b1 | b2

b1:
    v9: void = zeroed
    v10: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; } = variant.new 1
    store l1, v10
    jump b3

b2:
    v11: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v12: ref<String, managed, mutable, local> = load ((*v11).1 as 0)
    v13: ref<String, borrowed, 'a, readonly> = cast.bit v12 -> ref<String, borrowed, 'a, readonly>
    v14: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; } = variant.new 0, v13
    store l1, v14
    jump b3

b3:
    v15: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; } = load l1
    v16: test.main.Entry<'a> = aggregate (v4, v15)
    store l2, v16
    v17: ref<test.main.Entry<'a>, borrowed, 'frame, readonly> = address l2
    call test.main.write(v17): (ref<test.main.Entry<'a>, borrowed, 'frame, readonly>) => void
    return
}

/// @layout.struct name=test.main.Counter size=16 align=8
/// @layout.field owner=test.main.Counter index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Counter index=1 name=unit offset=8 size=8 align=8
/// @layout.struct name=test.main.Entry<'a> size=16 align=8
/// @layout.field owner=test.main.Entry<'a> index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Entry<'a> index=1 name=unit offset=8 size=8 align=8
/// @layout.variant name=type@22 size=8 align=8
/// @layout.discriminant owner=type@22 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@22 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@22 index=1 discriminant=1 payload_offset=0
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.write",
        r#"
type test.main.Entry<'a> {
    name: ref<String, borrowed, 'a, readonly>;
    unit: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; };
}

function test.main.write<'a, 'b>(v0: ref<test.main.Entry<'a>, borrowed, 'b, readonly>): void {
    local l0: ref<test.main.Entry<'a>, borrowed, 'b, readonly>

entry(v0: ref<test.main.Entry<'a>, borrowed, 'b, readonly>):
    store l0, v0
    return
}

/// @layout.struct name=test.main.Entry<'a> size=16 align=8
/// @layout.field owner=test.main.Entry<'a> index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Entry<'a> index=1 name=unit offset=8 size=8 align=8
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    name: ref<String, managed, mutable, local>;
    unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: ref<String, managed, mutable, local>, v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }): void {
    local l0: ref<String, managed, mutable, local>
    local l1: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }
    local l2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: ref<String, managed, mutable, local>, v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }):
    store l0, v1
    store l1, v2
    store l2, v0
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l2
    v4: ref<String, managed, mutable, local> = load l0
    store (*v3).0, v4
    v5: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l2
    v6: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load l1
    store (*v5).1, v6
    return
}

/// @layout.struct name=test.main.Counter size=16 align=8
/// @layout.field owner=test.main.Counter index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Counter index=1 name=unit offset=8 size=8 align=8
/// @layout.variant name=type@9 size=8 align=8
/// @layout.discriminant owner=type@9 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@9 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@9 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.ds", "test.main.Counter.add", r#"
@nocopy
type test.main.Counter {
    name: ref<String, managed, mutable, local>;
    unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

type test.main.Entry<'a> {
    name: ref<String, borrowed, 'a, readonly>;
    unit: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; };
}

function test.main.Counter.add<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly>): void {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly>
    local l1: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; }
    local l2: test.main.Entry<'a>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v1).0
    v3: ref<String, managed, mutable, local> = load (*v2)
    v4: ref<String, borrowed, 'a, readonly> = cast.bit v3 -> ref<String, borrowed, 'a, readonly>
    v5: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v6: uint1 = variant.tag.load (*v5).1
    v7: uint1 = 1
    v8: boolean = eq v6, v7
    branch v8 => b1 | b2

b1:
    v9: void = zeroed
    v10: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; } = variant.new 1
    store l1, v10
    jump b3

b2:
    v11: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v12: ref<String, managed, mutable, local> = load ((*v11).1 as 0)
    v13: ref<String, borrowed, 'a, readonly> = cast.bit v12 -> ref<String, borrowed, 'a, readonly>
    v14: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; } = variant.new 0, v13
    store l1, v14
    jump b3

b3:
    v15: variant<uint1> { 0uint1 = ref<String, borrowed, 'a, readonly>; 1uint1 = void; } = load l1
    v16: test.main.Entry<'a> = aggregate (v4, v15)
    store l2, v16
    v17: ref<test.main.Entry<'a>, borrowed, 'frame, readonly> = address l2
    call test.main.write(v17): (ref<test.main.Entry<'a>, borrowed, 'frame, readonly>) => void
    return
}

/// @layout.struct name=test.main.Counter size=16 align=8
/// @layout.field owner=test.main.Counter index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Counter index=1 name=unit offset=8 size=8 align=8
/// @layout.struct name=test.main.Entry<'a> size=16 align=8
/// @layout.field owner=test.main.Entry<'a> index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Entry<'a> index=1 name=unit offset=8 size=8 align=8
/// @layout.variant name=type@22 size=8 align=8
/// @layout.discriminant owner=type@22 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@22 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@22 index=1 discriminant=1 payload_offset=0
"#);
}

#[test]
fn test_lower_a_parameter_place_first_borrowed_inside_a_branch() {
    let session = TestSession::single(
        r#"
struct Meter {
    value: int32;
}

function read(meter: &readonly Meter): int32 {
    return meter.value;
}

function main(pick: boolean, meter: Meter): int32 {
    let first = if (pick) { read(&meter) } else { 0 };
    return first + read(&meter);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
type test.main.Meter {
    value: int32;
}

function test.main.read<'a>(v0: ref<test.main.Meter, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Meter, borrowed, 'a, readonly>

entry(v0: ref<test.main.Meter, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Meter, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.struct name=test.main.Meter size=4 align=4
/// @layout.field owner=test.main.Meter index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.main",
        r#"
type test.main.Meter {
    value: int32;
}

function test.main.main(v0: boolean, v1: test.main.Meter): int32 {
    local l0: boolean
    local l1: test.main.Meter
    local l2: int32
    local l3: int32

entry(v0: boolean, v1: test.main.Meter):
    store l0, v0
    store l1, v1
    v2: boolean = load l0
    branch v2 => b1 | b2

b1:
    v3: ref<test.main.Meter, borrowed, 'frame, mutable> = address l1
    v4: int32 = call test.main.read(v3): (ref<test.main.Meter, borrowed, 'frame, readonly>) => int32
    store l2, v4
    jump b3

b2:
    v5: int32 = 0
    store l2, v5
    jump b3

b3:
    v6: int32 = load l2
    store l3, v6
    v7: int32 = load l3
    v8: ref<test.main.Meter, borrowed, 'frame, mutable> = address l1
    v9: int32 = call test.main.read(v8): (ref<test.main.Meter, borrowed, 'frame, readonly>) => int32
    v10: int32 = add v7, v9
    return v10
}

/// @layout.struct name=test.main.Meter size=4 align=4
/// @layout.field owner=test.main.Meter index=0 name=value offset=0 size=4 align=4
"#,
    );
}

/// Evaluate an omitted field's initializer at the application's open argument.
#[test]
fn test_lower_an_omitted_field_initializer_at_an_open_argument() {
    let session = TestSession::single(
        r#"
import { Default, Phantom } from "destack:memory";

struct Equality<T> {
    private phantom: Phantom<T> = Phantom.new();
}

extension<T> of Equality<T> implements Default {
    static default(): ^this {
        Equality<T> {}
    }
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Equality.Default.default",
        r#"
type test.main.Equality<T> {
    phantom: void;
}

function test.main.Equality.Default.default<T>(): test.main.Equality<T> {
entry:
    call Phantom.new<T>(): () => void
    v0: void = zeroed
    v1: test.main.Equality<T> = aggregate (v0)
    return v1
}
"#,
    );
}

/// A provided optional field enters its union case before the aggregate stores it.
#[test]
fn test_lower_a_provided_optional_field_into_its_union_case() {
    let session = TestSession::single(
        r#"
struct Handle {
    id: int32;
}

struct Entry {
    directory?: Handle;
    depth: int32;
}

function at(directory: Handle): Entry {
    return Entry { directory, depth: 1 };
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.at",
        r#"
type test.main.Handle {
    id: int32;
}

type test.main.Entry {
    directory: variant<uint1> { 0uint1 = test.main.Handle; 1uint1 = void; };
    depth: int32;
}

function test.main.at(v0: test.main.Handle): test.main.Entry {
    local l0: test.main.Handle

entry(v0: test.main.Handle):
    store l0, v0
    v1: test.main.Handle = load l0
    v2: variant<uint1> { 0uint1 = test.main.Handle; 1uint1 = void; } = variant.new 0, v1
    v3: int32 = 1
    v4: test.main.Entry = aggregate (v2, v3)
    return v4
}

/// @layout.struct name=test.main.Handle size=4 align=4
/// @layout.field owner=test.main.Handle index=0 name=id offset=0 size=4 align=4
/// @layout.struct name=test.main.Entry size=12 align=4
/// @layout.field owner=test.main.Entry index=0 name=directory offset=0 size=8 align=4
/// @layout.field owner=test.main.Entry index=1 name=depth offset=8 size=4 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
"#,
    );
}

/// An index-signature alias has one representation in a parameter and in an optional field.
#[test]
fn test_lower_an_index_signature_alias_the_same_in_a_parameter_and_an_optional_field() {
    let session = TestSession::single(
        r#"
type Fields = { readonly [key: string]: int32 };

struct Entry {
    fields?: Fields | undefined;
    depth: int32;
}

function at(fields: Fields): Entry {
    return Entry { fields, depth: 1 };
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.at",
        r#"
type test.main.Entry {
    fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; };
    depth: int32;
}

function test.main.at(v0: dynamic<{  }, managed, mutable, local>): test.main.Entry {
    local l0: dynamic<{  }, managed, mutable, local>

entry(v0: dynamic<{  }, managed, mutable, local>):
    store l0, v0
    v1: dynamic<{  }, managed, mutable, local> = load l0
    v2: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v1
    v3: int32 = 1
    v4: test.main.Entry = aggregate (v2, v3)
    return v4
}

/// @layout.struct name=test.main.Entry size=24 align=8
/// @layout.field owner=test.main.Entry index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=test.main.Entry index=1 name=depth offset=16 size=4 align=4
/// @layout.struct name=type@1 size=0 align=1
/// @layout.variant name=type@5 size=16 align=8
/// @layout.discriminant owner=type@5 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@5 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@5 index=1 discriminant=1 payload_offset=0
"#,
    );
}

/// An optional field assigned through a place stores the value in its present case.
#[test]
fn test_assign_an_optional_field_into_its_union_case() {
    let session = TestSession::single(
        r#"
struct Entry {
    limit?: int32;
    depth: int32;
}

function bound(entry: &Entry, limit: int32): void {
    entry.limit = limit;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.bound",
        r#"
type test.main.Entry {
    limit: variant<uint1> { 0uint1 = int32; 1uint1 = void; };
    depth: int32;
}

function test.main.bound<'a>(v0: ref<test.main.Entry, borrowed, 'a, mutable>, v1: int32): void {
    local l0: ref<test.main.Entry, borrowed, 'a, mutable>
    local l1: int32

entry(v0: ref<test.main.Entry, borrowed, 'a, mutable>, v1: int32):
    store l0, v0
    store l1, v1
    v2: ref<test.main.Entry, borrowed, 'a, mutable> = load l0
    v3: int32 = load l1
    v4: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v3
    store (*v2).0, v4
    return
}

/// @layout.struct name=test.main.Entry size=12 align=4
/// @layout.field owner=test.main.Entry index=0 name=limit offset=0 size=8 align=4
/// @layout.field owner=test.main.Entry index=1 name=depth offset=8 size=4 align=4
/// @layout.variant name=type@4 size=8 align=4
/// @layout.discriminant owner=type@4 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@4 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@4 index=1 discriminant=1 payload_offset=4
"#,
    );
}
