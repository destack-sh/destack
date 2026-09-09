use crate::tests::TestSession;

#[test]
fn test_lower_imported_function_call_to_an_extern_symbol() {
    let session = TestSession::builder()
        .module(
            "math.ds",
            r#"
export function add(a: int32, b: int32): int32 {
    return a + b;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { add } from "./math";

function total(base: int32): int32 {
    return add(base, base);
}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.ds",
        "test.main.total",
        r#"
function test.main.total(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: int32 = local.get l0
    v3: int32 = call test.math.add(v1, v2): (int32, int32) => int32
    return v3
}
"#,
    );
}

#[test]
fn test_lower_imported_call_returning_a_foreign_struct() {
    let session = TestSession::builder()
        .module(
            "point.ds",
            r#"
export struct Point {
    x: int32;
    y: int32;
}

export function diagonal(a: int32, b: int32): Point {
    return Point { x: a, y: b };
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point, diagonal } from "./point";

function stretch(by: int32): int32 {
    let point: Point = diagonal(by, by);
    return point.x + point.y;
}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.ds",
        "test.main.stretch",
        r#"
@copy
type test.point.Point;

function test.main.stretch(v0: int32): int32 {
    local l0: int32
    local l1: test.point.Point

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: int32 = local.get l0
    v3: test.point.Point = call test.point.diagonal(v1, v2): (int32, int32) => test.point.Point
    local.set l1, v3
    v4: test.point.Point = local.get l1
    v5: int32 = field.get v4, 0
    v6: test.point.Point = local.get l1
    v7: int32 = field.get v6, 1
    v8: int32 = add v5, v7
    return v8
}
"#,
    );
}

#[test]
fn test_lower_imported_struct_method_call_through_an_extern() {
    let session = TestSession::builder()
        .module(
            "point.ds",
            r#"
export struct Point {
    x: int32;
    y: int32;

    length(): int32 {
        return this.x + this.y;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point } from "./point";

function measure(by: int32): int32 {
    let point = Point { x: by, y: by };
    return point.length();
}
"#,
        )
        .build();

    session.assert_mir_function("main.ds", "test.main.measure", r#"
@copy
type test.point.Point;

function test.main.measure(v0: int32): int32 {
    local l0: int32
    local l1: test.point.Point

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: int32 = local.get l0
    v3: test.point.Point = aggregate (v1, v2)
    local.set l1, v3
    v4: ref<test.point.Point, borrowed, 'frame, readonly, local> = local.address l1
    v5: int32 = call test.point.Point.length(v4): <'a>(ref<test.point.Point, borrowed, 'a, readonly, local>) => int32
    return v5
}
"#);
}

#[test]
fn test_lower_imported_class_construction_and_method_call() {
    let session = TestSession::builder()
        .module(
            "box.ds",
            r#"
export class Box {
    weight: int32 = 0;

    constructor(weight: int32) {
        this.weight = weight;
    }

    weigh(): int32 {
        return this.weight;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Box } from "./box";

function open(): int32 {
    const parcel = new Box(7);
    return parcel.weigh();
}
"#,
        )
        .build();

    session.assert_mir_function("main.ds", "test.main.open", r#"
type test.box.Box;

function test.main.open(): int32 {
    local l0: ref<test.box.Box, managed, mutable, local>

entry:
    v0: int32 = 7
    v1: ref<test.box.Box, managed, mutable, local> = new.zeroed test.box.Box
    v2: ref<uninit<test.box.Box>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.box.Box>, borrowed, 'managed, mutable, local>
    call test.box.Box.constructor(v2, v0): <'a>(ref<uninit<test.box.Box>, borrowed, 'a, mutable, local>, int32) => void
    local.set l0, v1
    v3: ref<test.box.Box, managed, mutable, local> = local.get l0
    v4: int32 = call test.box.Box.weigh(v3): (ref<test.box.Box, managed, mutable, local>) => int32
    return v4
}
"#);

    session.assert_mir_function("main.ds", "test.box.Box.constructor", r#"
type test.box.Box;

external function test.box.Box.constructor<'a>(ref<uninit<test.box.Box>, borrowed, 'a, mutable, local>, int32): void
"#);

    session.assert_mir_function(
        "main.ds",
        "test.box.Box.weigh",
        r#"
type test.box.Box;

external function test.box.Box.weigh(ref<test.box.Box, managed, mutable, local>): int32
"#,
    );
}

/// Lower a derived clone on a scalar to a synthesized receiver copy.
#[test]
fn test_lower_a_derived_clone_on_a_scalar() {
    let session = TestSession::single(
        r#"
function duplicate(value: int32): int32 {
    return value.clone();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.duplicate",
        r#"
function test.main.duplicate(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: ref<int32, borrowed, 'frame, readonly, local> = local.address l0
    v2: int32 = call Integer.Clone.clone<int32>(v1): <'a>(ref<int32, borrowed, 'a, readonly, local>) => int32
    return v2
}
"#,
    );
}

#[test]
fn test_lower_an_imported_generic_call_instantiating_a_nested_class() {
    let session = TestSession::builder()
        .module(
            "box.ds",
            r#"
export local class Holder<T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

export function hold<T: Copy>(value: T): Holder<T> {
    return new Holder(value);
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { hold } from "./box";

function go(): int32 {
    hold(1);

    return 0;
}
"#,
        )
        .build();

    session.assert_mir_function("main.ds", "test.main.go", r#"
type test.box.Holder<T: Copy>;

function test.main.go(): int32 {
entry:
    v0: int64 = 1
    v1: ref<test.box.Holder<int64>, managed, mutable, local> = call test.box.hold<int64>(v0): (int64) => ref<test.box.Holder<int64>, managed, mutable, local>
    v2: int32 = 0
    return v2
}
"#);

    session.assert_mir_function("main.ds", "test.box.hold<int64>", r#"
type test.box.Holder<T: Copy>;

shared function test.box.hold<int64>(v0: int64): ref<test.box.Holder<int64>, managed, mutable, local>;
"#);
}

#[test]
fn test_lower_an_imported_generic_call_closing_over_a_nested_class() {
    let session = TestSession::builder()
        .module(
            "box.ds",
            r#"
export local class Holder<T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

export function hold<T: Copy>(value: T): Holder<T> {
    let make: ^Function<(), Holder<T>, "once"> = () => new Holder(value);

    return make();
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { hold } from "./box";

function go(): int32 {
    hold(1);

    return 0;
}
"#,
        )
        .build();

    session.assert_mir_function("main.ds", "test.main.go", r#"
type test.box.Holder<T: Copy>;

function test.main.go(): int32 {
entry:
    v0: int64 = 1
    v1: ref<test.box.Holder<int64>, managed, mutable, local> = call test.box.hold<int64>(v0): (int64) => ref<test.box.Holder<int64>, managed, mutable, local>
    v2: int32 = 0
    return v2
}
"#);

    session.assert_mir_function("main.ds", "test.box.hold<int64>", r#"
type test.box.Holder<T: Copy>;

shared function test.box.hold<int64>(v0: int64): ref<test.box.Holder<int64>, managed, mutable, local>;
"#);
}

#[test]
fn test_lower_an_async_function_to_its_creation_entry() {
    let session = TestSession::single(
        r#"
async function fetchCount(): Promise<int32> {
    return 1;
}

async function double(): Promise<int32> {
    const count = await fetchCount();

    return count + count;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.fetchCount", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.fetchCount(): ref<Promise<int32>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable, local> = new.complete v0
    v2: function<() => int32, once, unique, mutable, local> = function.bind test.main.fetchCount.body, v1
    v3: ref<Promise<int32>, managed, mutable, local> = call Promise.create<int32>(v2): (function<() => int32, once, unique, mutable, local>) => ref<Promise<int32>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@109 size=0 align=1
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.fetchCount.body",
        r#"
@environment(ref<{  }, unique, mutable, local>)
function test.main.fetchCount.body(): int32 {
entry:
    v0: ref<{  }, unique, mutable, local> = function.environment.current
    v1: {  } = load v0
    release v0
    v2: int32 = 1
    return v2
}

/// @layout.struct name=type@109 size=0 align=1
"#,
    );

    session.assert_mir_function("main.ds", "test.main.double", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.double(): ref<Promise<int32>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable, local> = new.complete v0
    v2: function<() => int32, once, unique, mutable, local> = function.bind test.main.double.body, v1
    v3: ref<Promise<int32>, managed, mutable, local> = call Promise.create<int32>(v2): (function<() => int32, once, unique, mutable, local>) => ref<Promise<int32>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@109 size=0 align=1
"#);

    session.assert_mir_function("main.ds", "test.main.double.body", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{  }, unique, mutable, local>)
function test.main.double.body(): int32 {
    local l0: int32

entry:
    v0: ref<{  }, unique, mutable, local> = function.environment.current
    v1: {  } = load v0
    release v0
    v2: ref<Promise<int32>, managed, mutable, local> = call test.main.fetchCount(): () => ref<Promise<int32>, managed, mutable, local>
    v3: int32 = call Promise.park<int32, int32>(v2): (ref<Promise<int32>, managed, mutable, local>) => int32
    local.set l0, v3
    v4: int32 = local.get l0
    v5: int32 = local.get l0
    v6: int32 = add v4, v5
    return v6
}

/// @layout.struct name=type@109 size=0 align=1
"#);
}

#[test]
fn test_lower_a_generic_async_function_instance() {
    let session = TestSession::single(
        r#"
async function pass<T: Copy>(value: T): Promise<T> {
    return value;
}

function run(): void {
    pass(7);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.run", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.run(): void {
entry:
    v0: int64 = 7
    v1: ref<Promise<int64>, managed, mutable, local> = call test.main.pass<int64>(v0): (int64) => ref<Promise<int64>, managed, mutable, local>
    return
}
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.pass<int64>",
        r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

shared function test.main.pass<int64>(v0: int64): ref<Promise<int64>, managed, mutable, local>;
"#,
    );
}

#[test]
fn test_lower_a_generator_function_to_its_producer_body() {
    let session = TestSession::single(
        r#"
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}

function* tally(): Generator<int32, int32, int32> {
    const first = yield 1;
    const second = yield first;

    return first + second;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.count", r#"
@languageItem("async.Generator")
type Generator<Y, R, N>;

@copy
type GeneratorProducer<Y, R, N>;

function test.main.count(v0: int32): ref<Generator<int32, void, void>, managed, mutable, local> {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: { int32 } = aggregate (v1)
    v3: ref<{ int32 }, unique, mutable, local> = new.complete v2
    v4: function<(GeneratorProducer<int32, void, void>) => void, once, unique, mutable, local> = function.bind test.main.count.body, v3
    v5: ref<Generator<int32, void, void>, managed, mutable, local> = call Generator.create<int32, void, void>(v4): (function<(GeneratorProducer<int32, void, void>) => void, once, unique, mutable, local>) => ref<Generator<int32, void, void>, managed, mutable, local>
    return v5
}

/// @layout.struct name=type@186 size=4 align=4
/// @layout.field owner=type@186 index=0 offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.count.body", r#"
@copy
@languageItem("async.GeneratorNext")
type GeneratorNext<N>;

@copy
@languageItem("async.GeneratorReturn")
type GeneratorReturn<R>;

@copy
type GeneratorProducer<Y, R, N>;

@environment(ref<{ int32 }, unique, mutable, local>)
function test.main.count.body(v0: GeneratorProducer<int32, void, void>): void {
    local l0: int32
    local l1: GeneratorProducer<int32, void, void>
    local l2: int32

entry(v0: GeneratorProducer<int32, void, void>):
    v1: ref<{ int32 }, unique, mutable, local> = function.environment.current
    v2: { int32 } = load v1
    release v1
    v3: int32 = field.get v2, 0
    local.set l0, v3
    local.set l1, v0
    v4: int32 = 0
    local.set l2, v4
    jump b1

b1:
    v5: int32 = local.get l2
    v6: int32 = local.get l0
    v7: boolean = lt v5, v6
    branch v7 => b2 | b4

b2:
    v8: int32 = local.get l2
    v9: ref<GeneratorProducer<int32, void, void>, borrowed, 'l0, mutable, local> = local.address l1
    v10: variant<uint1> { 0uint1 = GeneratorNext<void>; 1uint1 = GeneratorReturn<void>; } = call GeneratorProducer.yield<int32, void, void>(v9, v8): <'a>(ref<GeneratorProducer<int32, void, void>, borrowed, 'a, mutable, local>, int32) => variant<uint1> { 0uint1 = GeneratorNext<void>; 1uint1 = GeneratorReturn<void>; }
    variant.switch v10, 0 => b5, 1 => b6

b3:
    v15: int32 = local.get l2
    v16: int32 = 1
    v17: int32 = add v15, v16
    local.set l2, v17
    jump b1

b4:
    return

b5:
    v13: GeneratorNext<void> = variant.payload v10, 0
    v14: variant<uint1> { 0uint1 = void; } = field.get v13, 1
    jump b7

b6:
    v11: GeneratorReturn<void> = variant.payload v10, 1
    v12: void = field.get v11, 1
    return

b7:
    jump b3
}

/// @layout.variant name=type@119 size=1 align=1
/// @layout.discriminant owner=type@119 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@119 index=0 discriminant=0 payload_offset=1
/// @layout.variant name=type@124 size=1 align=1
/// @layout.discriminant owner=type@124 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=1
/// @layout.case owner=type@124 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@124 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@186 size=4 align=4
/// @layout.field owner=type@186 index=0 offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.tally", r#"
@languageItem("async.Generator")
type Generator<Y, R, N>;

@copy
type GeneratorProducer<Y, R, N>;

function test.main.tally(): ref<Generator<int32, int32, int32>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable, local> = new.complete v0
    v2: function<(GeneratorProducer<int32, int32, int32>) => int32, once, unique, mutable, local> = function.bind test.main.tally.body, v1
    v3: ref<Generator<int32, int32, int32>, managed, mutable, local> = call Generator.create<int32, int32, int32>(v2): (function<(GeneratorProducer<int32, int32, int32>) => int32, once, unique, mutable, local>) => ref<Generator<int32, int32, int32>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@205 size=0 align=1
"#);

    session.assert_mir_function("main.ds", "test.main.tally.body", r#"
@copy
@languageItem("async.GeneratorNext")
type GeneratorNext<N>;

@copy
@languageItem("async.GeneratorReturn")
type GeneratorReturn<R>;

@copy
type GeneratorProducer<Y, R, N>;

@environment(ref<{  }, unique, mutable, local>)
function test.main.tally.body(v0: GeneratorProducer<int32, int32, int32>): int32 {
    local l0: GeneratorProducer<int32, int32, int32>
    local l1: int32, readonly
    local l2: int32
    local l3: int32, readonly
    local l4: int32

entry(v0: GeneratorProducer<int32, int32, int32>):
    v1: ref<{  }, unique, mutable, local> = function.environment.current
    v2: {  } = load v1
    release v1
    local.set l0, v0
    v3: int32 = 1
    v4: ref<GeneratorProducer<int32, int32, int32>, borrowed, 'l0, mutable, local> = local.address l0
    v5: variant<uint1> { 0uint1 = GeneratorNext<int32>; 1uint1 = GeneratorReturn<int32>; } = call GeneratorProducer.yield<int32, int32, int32>(v4, v3): <'a>(ref<GeneratorProducer<int32, int32, int32>, borrowed, 'a, mutable, local>, int32) => variant<uint1> { 0uint1 = GeneratorNext<int32>; 1uint1 = GeneratorReturn<int32>; }
    variant.switch v5, 0 => b1, 1 => b2

b1:
    v8: GeneratorNext<int32> = variant.payload v5, 0
    v9: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = field.get v8, 1
    v10: int32 = variant.payload v9, 1
    local.set l1, v10
    jump b3

b2:
    v6: GeneratorReturn<int32> = variant.payload v5, 1
    v7: int32 = field.get v6, 1
    return v7

b3:
    v11: int32 = local.get l1
    local.set l2, v11
    v12: int32 = local.get l2
    v13: ref<GeneratorProducer<int32, int32, int32>, borrowed, 'l0, mutable, local> = local.address l0
    v14: variant<uint1> { 0uint1 = GeneratorNext<int32>; 1uint1 = GeneratorReturn<int32>; } = call GeneratorProducer.yield<int32, int32, int32>(v13, v12): <'a>(ref<GeneratorProducer<int32, int32, int32>, borrowed, 'a, mutable, local>, int32) => variant<uint1> { 0uint1 = GeneratorNext<int32>; 1uint1 = GeneratorReturn<int32>; }
    variant.switch v14, 0 => b4, 1 => b5

b4:
    v17: GeneratorNext<int32> = variant.payload v14, 0
    v18: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = field.get v17, 1
    v19: int32 = variant.payload v18, 1
    local.set l3, v19
    jump b6

b5:
    v15: GeneratorReturn<int32> = variant.payload v14, 1
    v16: int32 = field.get v15, 1
    return v16

b6:
    v20: int32 = local.get l3
    local.set l4, v20
    v21: int32 = local.get l2
    v22: int32 = local.get l4
    v23: int32 = add v21, v22
    return v23
}

/// @layout.variant name=type@155 size=8 align=4
/// @layout.discriminant owner=type@155 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@155 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@155 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@160 size=12 align=4
/// @layout.discriminant owner=type@160 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@160 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@160 index=1 discriminant=1 payload_offset=4
/// @layout.struct name=type@205 size=0 align=1
"#);
}

#[test]
fn test_lower_an_async_lambda_capturing_its_outer_scope() {
    let session = TestSession::single(
        r#"
async function fetchCount(): Promise<int32> {
    return 1;
}

function makeAdder(base: int32): ^Function<(), Promise<int32>, "once"> {
    @capture("move")
    const add: ^Function<(), Promise<int32>, "once"> = async (): Promise<int32> => {
        const count = await fetchCount();

        return base + count;
    };

    return add;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.fetchCount", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.fetchCount(): ref<Promise<int32>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable, local> = new.complete v0
    v2: function<() => int32, once, unique, mutable, local> = function.bind test.main.fetchCount.body, v1
    v3: ref<Promise<int32>, managed, mutable, local> = call Promise.create<int32>(v2): (function<() => int32, once, unique, mutable, local>) => ref<Promise<int32>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@110 size=0 align=1
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.fetchCount.body",
        r#"
@environment(ref<{  }, unique, mutable, local>)
function test.main.fetchCount.body(): int32 {
entry:
    v0: ref<{  }, unique, mutable, local> = function.environment.current
    v1: {  } = load v0
    release v0
    v2: int32 = 1
    return v2
}

/// @layout.struct name=type@110 size=0 align=1
"#,
    );

    session.assert_mir_function("main.ds", "test.main.makeAdder", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.makeAdder(v0: int32): function<() => ref<Promise<int32>, managed, mutable, local>, once, unique, mutable, local> {
    local l0: int32
    local l1: function<() => ref<Promise<int32>, managed, mutable, local>, once, unique, mutable, local>

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: { int32 } = aggregate (v1)
    v3: ref<{ int32 }, unique, mutable, local> = new.complete v2
    v4: function<() => ref<Promise<int32>, managed, mutable, local>, once, unique, mutable, local> = function.bind test.main.makeAdder.closure#0, v3
    local.set l1, v4
    v5: function<() => ref<Promise<int32>, managed, mutable, local>, once, unique, mutable, local> = local.get l1
    return v5
}

/// @layout.struct name=type@127 size=4 align=4
/// @layout.field owner=type@127 index=0 offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.makeAdder.closure#0", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{ int32 }, unique, mutable, local>)
function test.main.makeAdder.closure#0(): ref<Promise<int32>, managed, mutable, local> {
entry:
    v0: ref<{ int32 }, unique, mutable, local> = function.environment.current
    v1: ref<{ int32 }, unique, mutable, local> = function.environment.current
    v2: { ref<{ int32 }, unique, mutable, local> } = aggregate (v1)
    v3: ref<{ ref<{ int32 }, unique, mutable, local> }, unique, mutable, local> = new.complete v2
    v4: function<() => int32, once, unique, mutable, local> = function.bind test.main.makeAdder.closure#0.body, v3
    v5: ref<Promise<int32>, managed, mutable, local> = call Promise.create<int32>(v4): (function<() => int32, once, unique, mutable, local>) => ref<Promise<int32>, managed, mutable, local>
    return v5
}

/// @layout.struct name=type@127 size=4 align=4
/// @layout.field owner=type@127 index=0 offset=0 size=4 align=4
/// @layout.struct name=type@147 size=8 align=8
/// @layout.field owner=type@147 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.ds", "test.main.makeAdder.closure#0.body", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{ ref<{ int32 }, unique, mutable, local> }, unique, mutable, local>)
function test.main.makeAdder.closure#0.body(): int32 {
    local l0: int32
    local l1: int32

entry:
    v0: ref<{ ref<{ int32 }, unique, mutable, local> }, unique, mutable, local> = function.environment.current
    v1: { ref<{ int32 }, unique, mutable, local> } = load v0
    release v0
    v2: ref<{ int32 }, unique, mutable, local> = field.get v1, 0
    v3: { int32 } = load v2
    release v2
    v4: int32 = field.get v3, 0
    local.set l0, v4
    v5: ref<Promise<int32>, managed, mutable, local> = call test.main.fetchCount(): () => ref<Promise<int32>, managed, mutable, local>
    v6: int32 = call Promise.park<int32, int32>(v5): (ref<Promise<int32>, managed, mutable, local>) => int32
    local.set l1, v6
    v7: int32 = local.get l0
    v8: int32 = local.get l1
    v9: int32 = add v7, v8
    return v9
}

/// @layout.struct name=type@127 size=4 align=4
/// @layout.field owner=type@127 index=0 offset=0 size=4 align=4
/// @layout.struct name=type@147 size=8 align=8
/// @layout.field owner=type@147 index=0 offset=0 size=8 align=8
"#);
}
