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
@copy
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
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.abscissa(v0: test.main.Point): int32 {
    local l0: test.main.Point

entry(v0: test.main.Point):
    local.set l0, v0
    v1: test.main.Point = local.get l0
    v2: int32 = field.get v1, 0
    return v2
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
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

@copy
type test.main.Segment {
    start: test.main.Point;
    end: test.main.Point;
}

function test.main.diagonal(v0: int32): test.main.Segment {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = 0
    v2: int32 = 0
    v3: test.main.Point = aggregate (v1, v2)
    v4: int32 = local.get l0
    v5: int32 = local.get l0
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
@copy
type test.main.Entry<'a> {
    name: slice<uint8, borrowed, 'a, mutable>;
}

function test.main.make<'a>(v0: slice<uint8, borrowed, 'a, mutable, local>): void {
    local l0: slice<uint8, borrowed, 'a, mutable, local>
    local l1: test.main.Entry<'a & local>

entry(v0: slice<uint8, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: slice<uint8, borrowed, 'a, mutable, local> = local.get l0
    v2: test.main.Entry<'a & local> = aggregate (v1)
    local.set l1, v2
    return
}

/// @layout.struct name=test.main.Entry<'a & local> size=16 align=8
/// @layout.field owner=test.main.Entry<'a & local> index=0 name=name offset=0 size=16 align=8
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
type test.main.Counter {
    total: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 0
    v3: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
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
type test.main.Counter {
    total: int32;
}

function test.main.Counter.read<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>): int32 {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<int32, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=total offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Counter.double",
        r#"
type test.main.Counter {
    total: int32;
}

function test.main.Counter.double<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>): int32 {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v2: int32 = call test.main.Counter.read(v1): <'a>(ref<test.main.Counter, borrowed, 'a, readonly, local>) => int32
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
@copy
type test.main.Options {
    count: int32;
    limit: variant<uint1> { 0uint1 = void; 1uint1 = int32; };
}

function test.main.make(): test.main.Options {
entry:
    v0: int32 = 1
    v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 0
    v2: test.main.Options = aggregate (v0, v1)
    return v2
}

/// @layout.struct name=test.main.Options size=12 align=4
/// @layout.field owner=test.main.Options index=0 name=count offset=0 size=4 align=4
/// @layout.field owner=test.main.Options index=1 name=limit offset=4 size=8 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.make",
        r#"
@copy
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
@copy
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
@copy
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
@languageItem("string.String")
type String;

type test.main.Counter {
    name: ref<String, managed, mutable, local>;
    unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@copy
type test.main.Entry<'a> {
    name: ref<String, borrowed, 'a, readonly>;
    unit: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly>; };
}

function test.main.Counter.add<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>): void {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly, local>
    local l1: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly, local>; }
    local l2: test.main.Entry<'a & local>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: ref<String, managed, readonly, local> = load v2
    v4: ref<String, borrowed, 'a, readonly, local> = cast.bit v3 -> ref<String, borrowed, 'a, readonly, local>
    v5: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v6: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }, borrowed, 'a, readonly, local> = field.project v5, 1
    v7: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; } = load v6
    v8: uint1 = variant.tag v7
    v9: uint1 = 0
    v10: boolean = eq v8, v9
    branch v10 => b1 | b2

b1:
    v11: void = zeroed
    v12: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly, local>; } = variant.new 0
    local.set l1, v12
    jump b3

b2:
    v13: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v14: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }, borrowed, 'a, readonly, local> = field.project v13, 1
    v15: uint1 = variant.tag.load v14
    switch v15, b5, 1 => b4

b3:
    v20: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly, local>; } = local.get l1
    v21: test.main.Entry<'a & local> = aggregate (v4, v20)
    local.set l2, v21
    v22: ref<test.main.Entry<'a & local>, borrowed, 'frame, readonly, local> = local.address l2
    call test.main.write(v22): <'a, 'b>(ref<test.main.Entry<'a & local>, borrowed, 'b, readonly, local>) => void
    return

b4:
    v16: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = variant.payload.project v14, 1
    v17: ref<String, managed, readonly, local> = load v16
    v18: ref<String, borrowed, 'a, readonly, local> = cast.bit v17 -> ref<String, borrowed, 'a, readonly, local>
    v19: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly, local>; } = variant.new 1, v18
    local.set l1, v19
    jump b3

b5:
    panic
}

/// @layout.struct name=test.main.Counter size=16 align=8
/// @layout.field owner=test.main.Counter index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Counter index=1 name=unit offset=8 size=8 align=8
/// @layout.struct name=test.main.Entry<'a & local> size=16 align=8
/// @layout.field owner=test.main.Entry<'a & local> index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Entry<'a & local> index=1 name=unit offset=8 size=8 align=8
/// @layout.variant name=type@24 size=8 align=8
/// @layout.discriminant owner=type@24 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@24 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@24 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@69 size=8 align=8
/// @layout.discriminant owner=type@69 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@69 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@69 index=1 discriminant=1 payload_offset=0
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.write",
        r#"
@copy
type test.main.Entry<'a> {
    name: ref<String, borrowed, 'a, readonly>;
    unit: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly>; };
}

function test.main.write<'a, 'b>(v0: ref<test.main.Entry<'a & local>, borrowed, 'b, readonly, local>): void {
    local l0: ref<test.main.Entry<'a & local>, borrowed, 'b, readonly, local>

entry(v0: ref<test.main.Entry<'a & local>, borrowed, 'b, readonly, local>):
    local.set l0, v0
    return
}

/// @layout.struct name=test.main.Entry<'a & local> size=16 align=8
/// @layout.field owner=test.main.Entry<'a & local> index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Entry<'a & local> index=1 name=unit offset=8 size=8 align=8
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
@languageItem("string.String")
type String;

type test.main.Counter {
    name: ref<String, managed, mutable, local>;
    unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: ref<String, managed, mutable, local>, v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }): void {
    local l0: ref<String, managed, mutable, local>
    local l1: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }
    local l2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: ref<String, managed, mutable, local>, v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }):
    local.set l0, v1
    local.set l1, v2
    local.set l2, v0
    v3: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l2
    v4: ref<String, managed, mutable, local> = local.get l0
    v5: ref<uninit<ref<String, managed, mutable, local>>, borrowed, 'a, mutable, local> = field.project v3, 0
    store v5, v4
    v6: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l2
    v7: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = local.get l1
    v8: ref<uninit<variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }>, borrowed, 'a, mutable, local> = field.project v6, 1
    store v8, v7
    return
}

/// @layout.struct name=test.main.Counter size=16 align=8
/// @layout.field owner=test.main.Counter index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Counter index=1 name=unit offset=8 size=8 align=8
/// @layout.variant name=type@11 size=8 align=8
/// @layout.discriminant owner=type@11 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.ds", "test.main.Counter.add", r#"
@languageItem("string.String")
type String;

type test.main.Counter {
    name: ref<String, managed, mutable, local>;
    unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@copy
type test.main.Entry<'a> {
    name: ref<String, borrowed, 'a, readonly>;
    unit: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly>; };
}

function test.main.Counter.add<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>): void {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly, local>
    local l1: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly, local>; }
    local l2: test.main.Entry<'a & local>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: ref<String, managed, readonly, local> = load v2
    v4: ref<String, borrowed, 'a, readonly, local> = cast.bit v3 -> ref<String, borrowed, 'a, readonly, local>
    v5: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v6: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }, borrowed, 'a, readonly, local> = field.project v5, 1
    v7: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; } = load v6
    v8: uint1 = variant.tag v7
    v9: uint1 = 0
    v10: boolean = eq v8, v9
    branch v10 => b1 | b2

b1:
    v11: void = zeroed
    v12: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly, local>; } = variant.new 0
    local.set l1, v12
    jump b3

b2:
    v13: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v14: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }, borrowed, 'a, readonly, local> = field.project v13, 1
    v15: uint1 = variant.tag.load v14
    switch v15, b5, 1 => b4

b3:
    v20: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly, local>; } = local.get l1
    v21: test.main.Entry<'a & local> = aggregate (v4, v20)
    local.set l2, v21
    v22: ref<test.main.Entry<'a & local>, borrowed, 'frame, readonly, local> = local.address l2
    call test.main.write(v22): <'a, 'b>(ref<test.main.Entry<'a & local>, borrowed, 'b, readonly, local>) => void
    return

b4:
    v16: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = variant.payload.project v14, 1
    v17: ref<String, managed, readonly, local> = load v16
    v18: ref<String, borrowed, 'a, readonly, local> = cast.bit v17 -> ref<String, borrowed, 'a, readonly, local>
    v19: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly, local>; } = variant.new 1, v18
    local.set l1, v19
    jump b3

b5:
    panic
}

/// @layout.struct name=test.main.Counter size=16 align=8
/// @layout.field owner=test.main.Counter index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Counter index=1 name=unit offset=8 size=8 align=8
/// @layout.struct name=test.main.Entry<'a & local> size=16 align=8
/// @layout.field owner=test.main.Entry<'a & local> index=0 name=name offset=0 size=8 align=8
/// @layout.field owner=test.main.Entry<'a & local> index=1 name=unit offset=8 size=8 align=8
/// @layout.variant name=type@24 size=8 align=8
/// @layout.discriminant owner=type@24 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@24 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@24 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@69 size=8 align=8
/// @layout.discriminant owner=type@69 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@69 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@69 index=1 discriminant=1 payload_offset=0
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
@copy
type test.main.Meter {
    value: int32;
}

function test.main.read<'a>(v0: ref<test.main.Meter, borrowed, 'a, readonly, local>): int32 {
    local l0: ref<test.main.Meter, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Meter, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Meter, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<int32, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=test.main.Meter size=4 align=4
/// @layout.field owner=test.main.Meter index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.main",
        r#"
@copy
type test.main.Meter {
    value: int32;
}

function test.main.main(v0: boolean, v1: test.main.Meter): int32 {
    local l0: boolean
    local l1: test.main.Meter
    local l2: int32
    local l3: int32

entry(v0: boolean, v1: test.main.Meter):
    local.set l0, v0
    local.set l1, v1
    v2: boolean = local.get l0
    branch v2 => b1 | b2

b1:
    v3: ref<test.main.Meter, borrowed, 'frame, mutable, local> = local.address l1
    v4: int32 = call test.main.read(v3): <'a>(ref<test.main.Meter, borrowed, 'a, readonly, local>) => int32
    local.set l2, v4
    jump b3

b2:
    v5: int32 = 0
    local.set l2, v5
    jump b3

b3:
    v6: int32 = local.get l2
    local.set l3, v6
    v7: int32 = local.get l3
    v8: ref<test.main.Meter, borrowed, 'frame, mutable, local> = local.address l1
    v9: int32 = call test.main.read(v8): <'a>(ref<test.main.Meter, borrowed, 'a, readonly, local>) => int32
    v10: int32 = add v7, v9
    return v10
}

/// @layout.struct name=test.main.Meter size=4 align=4
/// @layout.field owner=test.main.Meter index=0 name=value offset=0 size=4 align=4
"#,
    );
}
