use crate::tests::TestSession;

#[test]
fn test_lower_imported_function_call_to_an_extern_symbol() {
    let session = TestSession::builder()
        .module(
            "math.tspp",
            r#"
export function add(a: int32, b: int32): int32 {
    return a + b;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { add } from "./math";

function total(base: int32): int32 {
    return add(base, base);
}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.tspp",
        "test.main.total",
        r#"
function test.main.total(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = load l0
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
            "point.tspp",
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
            "main.tspp",
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
        "main.tspp",
        "test.main.stretch",
        r#"
type test.point.Point;

function test.main.stretch(v0: int32): int32 {
    local l0: int32
    local l1: test.point.Point

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = load l0
    v3: test.point.Point = call test.point.diagonal(v1, v2): (int32, int32) => test.point.Point
    store l1, v3
    v4: int32 = load (l1).0
    v5: int32 = load (l1).1
    v6: int32 = add v4, v5
    return v6
}
"#,
    );
}

#[test]
fn test_lower_imported_struct_method_call_through_an_extern() {
    let session = TestSession::builder()
        .module(
            "point.tspp",
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
            "main.tspp",
            r#"
import { Point } from "./point";

function measure(by: int32): int32 {
    let point = Point { x: by, y: by };
    return point.length();
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.measure", r#"
type test.point.Point;

function test.main.measure(v0: int32): int32 {
    local l0: int32
    local l1: test.point.Point

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = load l0
    v3: test.point.Point = aggregate (v1, v2)
    store l1, v3
    v4: ref<test.point.Point, borrowed, 'frame, readonly> = address l1
    v5: int32 = call test.point.Point.length(v4): (ref<test.point.Point, borrowed, 'frame, readonly>) => int32
    return v5
}
"#);
}

#[test]
fn test_lower_imported_class_construction_and_method_call() {
    let session = TestSession::builder()
        .module(
            "box.tspp",
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
            "main.tspp",
            r#"
import { Box } from "./box";

function open(): int32 {
    const parcel = new Box(7);
    return parcel.weigh();
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.open", r#"
@nocopy
type test.box.Box;

function test.main.open(): int32 {
    local l0: ref<test.box.Box, managed, mutable, local>

entry:
    v0: int32 = 7
    v1: ref<test.box.Box, managed, mutable, local> = new.zeroed test.box.Box, local
    v2: ref<uninit<test.box.Box>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.box.Box>, borrowed, 'managed, mutable>
    call test.box.Box.constructor(v2, v0): (ref<uninit<test.box.Box>, borrowed, 'managed, mutable>, int32) => void
    store l0, v1
    v3: ref<test.box.Box, managed, mutable, local> = load l0
    v4: int32 = call test.box.Box.weigh(v3): (ref<test.box.Box, managed, mutable, local>) => int32
    return v4
}
"#);

    session.assert_mir_function("main.tspp", "test.box.Box.constructor", r#"
@nocopy
type test.box.Box;

external constructor test.box.Box.constructor(ref<uninit<test.box.Box>, borrowed, 'managed, mutable>, int32): void
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.box.Box.weigh",
        r#"
@nocopy
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
        "main.tspp",
        "test.main.duplicate",
        r#"
function test.main.duplicate(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: ref<int32, borrowed, 'frame, immutable> = address l0
    v2: int32 = call Integer.Clone.clone<int32>(v1): (ref<int32, borrowed, 'frame, immutable>) => int32
    return v2
}
"#,
    );
}

#[test]
fn test_lower_an_imported_generic_call_instantiating_a_nested_class() {
    let session = TestSession::builder()
        .module(
            "box.tspp",
            r#"
export class Holder<T: Copy> {
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
            "main.tspp",
            r#"
import { hold } from "./box";

function go(): int32 {
    hold(1);

    return 0;
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.go", r#"
@nocopy
type test.box.Holder<T: Copy>;

function test.main.go(): int32 {
entry:
    v0: int64 = 1
    v1: ref<test.box.Holder<int64>, managed, mutable, local> = call test.box.hold<int64>(v0): (int64) => ref<test.box.Holder<int64>, managed, mutable, local>
    v2: int32 = 0
    return v2
}
"#);

    session.assert_mir_function("main.tspp", "test.box.hold<int64>", r#"
@nocopy
type test.box.Holder<T: Copy>;

shared function test.box.hold<int64>(v0: int64): ref<test.box.Holder<int64>, managed, mutable, local>;
"#);
}

#[test]
fn test_lower_an_imported_generic_call_closing_over_a_nested_class() {
    let session = TestSession::builder()
        .module(
            "box.tspp",
            r#"
export class Holder<T: Copy> {
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
            "main.tspp",
            r#"
import { hold } from "./box";

function go(): int32 {
    hold(1);

    return 0;
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.go", r#"
@nocopy
type test.box.Holder<T: Copy>;

function test.main.go(): int32 {
entry:
    v0: int64 = 1
    v1: ref<test.box.Holder<int64>, managed, mutable, local> = call test.box.hold<int64>(v0): (int64) => ref<test.box.Holder<int64>, managed, mutable, local>
    v2: int32 = 0
    return v2
}
"#);

    session.assert_mir_function("main.tspp", "test.box.hold<int64>", r#"
@nocopy
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

    session.assert_mir_function("main.tspp", "test.main.fetchCount", r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.fetchCount(): ref<Promise<int32>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable> = new.complete v0
    v2: function<() => int32, once, unique, mutable> = function.bind test.main.fetchCount.body, v1
    v3: ref<Promise<int32>, managed, mutable, local> = call Promise.create<int32>(v2): (function<() => int32, once, unique, mutable>) => ref<Promise<int32>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@6 size=0 align=1
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.fetchCount.body",
        r#"
@environment(ref<{  }, unique, mutable>)
function test.main.fetchCount.body(): int32 {
entry:
    v0: ref<{  }, unique, mutable> = function.environment.current
    v1: {  } = load (*v0)
    v2: ref<uninit<{  }>, unique, mutable> = cast.bit v0 -> ref<uninit<{  }>, unique, mutable>
    release v2
    v3: int32 = 1
    return v3
}

/// @layout.struct name=type@6 size=0 align=1
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.double", r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.double(): ref<Promise<int32>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable> = new.complete v0
    v2: function<() => int32, once, unique, mutable> = function.bind test.main.double.body, v1
    v3: ref<Promise<int32>, managed, mutable, local> = call Promise.create<int32>(v2): (function<() => int32, once, unique, mutable>) => ref<Promise<int32>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@6 size=0 align=1
"#);

    session.assert_mir_function("main.tspp", "test.main.double.body", r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{  }, unique, mutable>)
function test.main.double.body(): int32 {
    local l0: int32

entry:
    v0: ref<{  }, unique, mutable> = function.environment.current
    v1: {  } = load (*v0)
    v2: ref<uninit<{  }>, unique, mutable> = cast.bit v0 -> ref<uninit<{  }>, unique, mutable>
    release v2
    v3: ref<Promise<int32>, managed, mutable, local> = call test.main.fetchCount(): () => ref<Promise<int32>, managed, mutable, local>
    v4: int32 = call Promise.park<int32, int32>(v3): (ref<Promise<int32>, managed, mutable, local>) => int32
    store l0, v4
    v5: int32 = load l0
    v6: int32 = load l0
    v7: int32 = add v5, v6
    return v7
}

/// @layout.struct name=type@6 size=0 align=1
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

    session.assert_mir_function("main.tspp", "test.main.run", r#"
@nocopy
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
        "main.tspp",
        "test.main.pass<int64>",
        r#"
@nocopy
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

    session.assert_mir_function("main.tspp", "test.main.count", r#"
@nocopy
@languageItem("async.Generator")
type Generator<Y, R, N>;

type GeneratorProducer<Y, R, N>;

function test.main.count(v0: int32): ref<Generator<int32, void, void>, managed, mutable, local> {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: { int32 } = aggregate (v1)
    v3: ref<{ int32 }, unique, mutable> = new.complete v2
    v4: function<(GeneratorProducer<int32, void, void>) => void, once, unique, mutable> = function.bind test.main.count.body, v3
    v5: ref<Generator<int32, void, void>, managed, mutable, local> = call Generator.create<int32, void, void>(v4): (function<(GeneratorProducer<int32, void, void>) => void, once, unique, mutable>) => ref<Generator<int32, void, void>, managed, mutable, local>
    return v5
}

/// @layout.struct name=type@72 size=4 align=4
/// @layout.field owner=type@72 index=0 offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.count.body", r#"
@languageItem("async.GeneratorNext")
type GeneratorNext<N>;

@languageItem("async.GeneratorReturn")
type GeneratorReturn<R>;

type GeneratorProducer<Y, R, N>;

@environment(ref<{ int32 }, unique, mutable>)
function test.main.count.body(v0: GeneratorProducer<int32, void, void>): void {
    local l0: int32
    local l1: GeneratorProducer<int32, void, void>
    local l2: int32

entry(v0: GeneratorProducer<int32, void, void>):
    v1: ref<{ int32 }, unique, mutable> = function.environment.current
    v2: { int32 } = load (*v1)
    v3: ref<uninit<{ int32 }>, unique, mutable> = cast.bit v1 -> ref<uninit<{ int32 }>, unique, mutable>
    release v3
    v4: int32 = field.get v2, 0
    store l0, v4
    store l1, v0
    v5: int32 = 0
    store l2, v5
    jump b1

b1:
    v6: int32 = load l2
    v7: int32 = load l0
    v8: boolean = lt v6, v7
    branch v8 => b2 | b4

b2:
    v9: int32 = load l2
    v10: ref<GeneratorProducer<int32, void, void>, borrowed, 'l0, mutable> = address l1
    v11: variant<uint1> { 0uint1 = GeneratorNext<void>; 1uint1 = GeneratorReturn<void>; } = call GeneratorProducer.yield<int32, void, void>(v10, v9): <'a>(ref<GeneratorProducer<int32, void, void>, borrowed, 'a, mutable>, int32) => variant<uint1> { 0uint1 = GeneratorNext<void>; 1uint1 = GeneratorReturn<void>; }
    variant.switch v11, 0 => b5, 1 => b6

b3:
    v16: int32 = load l2
    v17: int32 = 1
    v18: int32 = add v16, v17
    store l2, v18
    jump b1

b4:
    return

b5:
    v14: GeneratorNext<void> = variant.payload v11, 0
    v15: variant<uint1> { 0uint1 = void; 1uint1 = void; } = field.get v14, 1
    jump b7

b6:
    v12: GeneratorReturn<void> = variant.payload v11, 1
    v13: void = field.get v12, 1
    return

b7:
    jump b3
}

/// @layout.struct name=type@72 size=4 align=4
/// @layout.field owner=type@72 index=0 offset=0 size=4 align=4
/// @layout.variant name=type@92 size=1 align=1
/// @layout.discriminant owner=type@92 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@92 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@92 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@95 size=1 align=1
/// @layout.discriminant owner=type@95 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@95 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@95 index=1 discriminant=1 payload_offset=1
"#);

    session.assert_mir_function("main.tspp", "test.main.tally", r#"
@nocopy
@languageItem("async.Generator")
type Generator<Y, R, N>;

type GeneratorProducer<Y, R, N>;

function test.main.tally(): ref<Generator<int32, int32, int32>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable> = new.complete v0
    v2: function<(GeneratorProducer<int32, int32, int32>) => int32, once, unique, mutable> = function.bind test.main.tally.body, v1
    v3: ref<Generator<int32, int32, int32>, managed, mutable, local> = call Generator.create<int32, int32, int32>(v2): (function<(GeneratorProducer<int32, int32, int32>) => int32, once, unique, mutable>) => ref<Generator<int32, int32, int32>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@11 size=0 align=1
"#);

    session.assert_mir_function("main.tspp", "test.main.tally.body", r#"
@languageItem("async.GeneratorNext")
type GeneratorNext<N>;

@languageItem("async.GeneratorReturn")
type GeneratorReturn<R>;

type GeneratorProducer<Y, R, N>;

@environment(ref<{  }, unique, mutable>)
function test.main.tally.body(v0: GeneratorProducer<int32, int32, int32>): int32 {
    local l0: GeneratorProducer<int32, int32, int32>
    local l1: int32, readonly
    local l2: int32
    local l3: int32, readonly
    local l4: int32

entry(v0: GeneratorProducer<int32, int32, int32>):
    v1: ref<{  }, unique, mutable> = function.environment.current
    v2: {  } = load (*v1)
    v3: ref<uninit<{  }>, unique, mutable> = cast.bit v1 -> ref<uninit<{  }>, unique, mutable>
    release v3
    store l0, v0
    v4: int32 = 1
    v5: ref<GeneratorProducer<int32, int32, int32>, borrowed, 'l0, mutable> = address l0
    v6: variant<uint1> { 0uint1 = GeneratorNext<int32>; 1uint1 = GeneratorReturn<int32>; } = call GeneratorProducer.yield<int32, int32, int32>(v5, v4): <'a>(ref<GeneratorProducer<int32, int32, int32>, borrowed, 'a, mutable>, int32) => variant<uint1> { 0uint1 = GeneratorNext<int32>; 1uint1 = GeneratorReturn<int32>; }
    variant.switch v6, 0 => b1, 1 => b2

b1:
    v9: GeneratorNext<int32> = variant.payload v6, 0
    v10: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = field.get v9, 1
    store l1, v10
    jump b3

b2:
    v7: GeneratorReturn<int32> = variant.payload v6, 1
    v8: int32 = field.get v7, 1
    return v8

b3:
    v11: int32 = load l1
    store l2, v11
    v12: int32 = load l2
    v13: ref<GeneratorProducer<int32, int32, int32>, borrowed, 'l0, mutable> = address l0
    v14: variant<uint1> { 0uint1 = GeneratorNext<int32>; 1uint1 = GeneratorReturn<int32>; } = call GeneratorProducer.yield<int32, int32, int32>(v13, v12): <'a>(ref<GeneratorProducer<int32, int32, int32>, borrowed, 'a, mutable>, int32) => variant<uint1> { 0uint1 = GeneratorNext<int32>; 1uint1 = GeneratorReturn<int32>; }
    variant.switch v14, 0 => b4, 1 => b5

b4:
    v17: GeneratorNext<int32> = variant.payload v14, 0
    v18: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = field.get v17, 1
    store l3, v18
    jump b6

b5:
    v15: GeneratorReturn<int32> = variant.payload v14, 1
    v16: int32 = field.get v15, 1
    return v16

b6:
    v19: int32 = load l3
    store l4, v19
    v20: int32 = load l2
    v21: int32 = load l4
    v22: int32 = add v20, v21
    return v22
}

/// @layout.struct name=type@11 size=0 align=1
/// @layout.variant name=type@102 size=12 align=4
/// @layout.discriminant owner=type@102 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@102 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@102 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@105 size=8 align=4
/// @layout.discriminant owner=type@105 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@105 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@105 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_function("main.tspp", "test.main.fetchCount", r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.fetchCount(): ref<Promise<int32>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable> = new.complete v0
    v2: function<() => int32, once, unique, mutable> = function.bind test.main.fetchCount.body, v1
    v3: ref<Promise<int32>, managed, mutable, local> = call Promise.create<int32>(v2): (function<() => int32, once, unique, mutable>) => ref<Promise<int32>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@6 size=0 align=1
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.fetchCount.body",
        r#"
@environment(ref<{  }, unique, mutable>)
function test.main.fetchCount.body(): int32 {
entry:
    v0: ref<{  }, unique, mutable> = function.environment.current
    v1: {  } = load (*v0)
    v2: ref<uninit<{  }>, unique, mutable> = cast.bit v0 -> ref<uninit<{  }>, unique, mutable>
    release v2
    v3: int32 = 1
    return v3
}

/// @layout.struct name=type@6 size=0 align=1
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.makeAdder", r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.makeAdder(v0: int32): function<() => ref<Promise<int32>, managed, mutable, local>, once, unique, mutable> {
    local l0: int32
    local l1: function<() => ref<Promise<int32>, managed, mutable, local>, once, unique, mutable>

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: { int32 } = aggregate (v1)
    v3: ref<{ int32 }, unique, mutable> = new.complete v2
    v4: function<() => ref<Promise<int32>, managed, mutable, local>, repeatable, unique, mutable> = function.bind test.main.makeAdder.closure#0, v3
    store l1, v4
    v5: function<() => ref<Promise<int32>, managed, mutable, local>, once, unique, mutable> = load l1
    return v5
}

/// @layout.struct name=type@56 size=4 align=4
/// @layout.field owner=type@56 index=0 offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.makeAdder.closure#0", r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{ int32 }, unique, mutable>)
function test.main.makeAdder.closure#0(): ref<Promise<int32>, managed, mutable, local> {
entry:
    v0: ref<{ int32 }, unique, mutable> = function.environment.current
    v1: ref<{ int32 }, unique, mutable> = function.environment.current
    v2: { ref<{ int32 }, unique, mutable> } = aggregate (v1)
    v3: ref<{ ref<{ int32 }, unique, mutable> }, unique, mutable> = new.complete v2
    v4: function<() => int32, once, unique, mutable> = function.bind test.main.makeAdder.closure#0.body, v3
    v5: ref<Promise<int32>, managed, mutable, local> = call Promise.create<int32>(v4): (function<() => int32, once, unique, mutable>) => ref<Promise<int32>, managed, mutable, local>
    return v5
}

/// @layout.struct name=type@56 size=4 align=4
/// @layout.field owner=type@56 index=0 offset=0 size=4 align=4
/// @layout.struct name=type@61 size=8 align=8
/// @layout.field owner=type@61 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.tspp", "test.main.makeAdder.closure#0.body", r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{ ref<{ int32 }, unique, mutable> }, unique, mutable>)
function test.main.makeAdder.closure#0.body(): int32 {
    local l0: int32
    local l1: int32

entry:
    v0: ref<{ ref<{ int32 }, unique, mutable> }, unique, mutable> = function.environment.current
    v1: { ref<{ int32 }, unique, mutable> } = load (*v0)
    v2: ref<uninit<{ ref<{ int32 }, unique, mutable> }>, unique, mutable> = cast.bit v0 -> ref<uninit<{ ref<{ int32 }, unique, mutable> }>, unique, mutable>
    release v2
    v3: ref<{ int32 }, unique, mutable> = field.get v1, 0
    v4: { int32 } = load (*v3)
    v5: ref<uninit<{ int32 }>, unique, mutable> = cast.bit v3 -> ref<uninit<{ int32 }>, unique, mutable>
    release v5
    v6: int32 = field.get v4, 0
    store l0, v6
    v7: ref<Promise<int32>, managed, mutable, local> = call test.main.fetchCount(): () => ref<Promise<int32>, managed, mutable, local>
    v8: int32 = call Promise.park<int32, int32>(v7): (ref<Promise<int32>, managed, mutable, local>) => int32
    store l1, v8
    v9: int32 = load l0
    v10: int32 = load l1
    v11: int32 = add v9, v10
    return v11
}

/// @layout.struct name=type@56 size=4 align=4
/// @layout.field owner=type@56 index=0 offset=0 size=4 align=4
/// @layout.struct name=type@61 size=8 align=8
/// @layout.field owner=type@61 index=0 offset=0 size=8 align=8
"#);
}
