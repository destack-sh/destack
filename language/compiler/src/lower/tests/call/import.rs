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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.total(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call test.math.add(v0, v0): (int32, int32) => int32
    return v1
}

external function test.math.add(int32, int32): int32
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type test.point.Point {
    x: int32;
    y: int32;
}

function test.main.stretch(v0: int32): int32 {
    local l0: test.point.Point

entry(v0: int32):
    v1: test.point.Point = call test.point.diagonal(v0, v0): (int32, int32) => test.point.Point
    local.set l0, v1
    v2: test.point.Point = local.get l0
    v3: int32 = field.get v2, 0
    v4: test.point.Point = local.get l0
    v5: int32 = field.get v4, 1
    v6: int32 = add v3, v5
    return v6
}

external function test.point.diagonal(int32, int32): test.point.Point

/// @layout.struct name=test.point.Point size=8 align=4
/// @layout.field owner=test.point.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.point.Point index=1 name=y offset=4 size=4 align=4
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type test.point.Point {
    x: int32;
    y: int32;
}

function test.main.measure(v0: int32): int32 {
    local l0: test.point.Point

entry(v0: int32):
    v1: test.point.Point = aggregate (v0, v0)
    local.set l0, v1
    v2: ref<test.point.Point, borrowed, 'frame, readonly, local> = local.address l0
    v3: int32 = call test.point.Point.length(v2): <'a>(ref<test.point.Point, borrowed, 'a, readonly, local>) => int32
    return v3
}

function test.point.Point.length<'a>(v0: ref<test.point.Point, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<test.point.Point, borrowed, 'a, readonly, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, readonly, local> = field.address v0, 1
    v4: int32 = load v3
    v5: int32 = add v2, v4
    return v5
}

/// @layout.struct name=test.point.Point size=8 align=4
/// @layout.field owner=test.point.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.point.Point index=1 name=y offset=4 size=4 align=4
"#,
    );
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
type test.box.Box {
    weight: int32;
}

function test.main.open(): int32 {
entry:
    v0: int32 = 7
    v1: ref<test.box.Box, managed, mutable, local> = new.zeroed test.box.Box
    v2: ref<uninit<test.box.Box>, borrowed, exclusive, local> = cast.bit v1 -> ref<uninit<test.box.Box>, borrowed, exclusive, local>
    call test.box.Box.constructor(v2, v0): (ref<uninit<test.box.Box>, borrowed, exclusive, local>, int32) => void
    v3: int32 = call test.box.Box.weigh(v1): (ref<test.box.Box, managed, mutable, local>) => int32
    return v3
}

function test.box.Box.constructor(v0: ref<uninit<test.box.Box>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<test.box.Box>, borrowed, exclusive, local>, v1: int32):
    v2: int32 = 0
    v3: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v3, v2
    v4: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v4, v1
    return
}

function test.box.Box.weigh(v0: ref<test.box.Box, managed, mutable, local>): int32 {
entry(v0: ref<test.box.Box, managed, mutable, local>):
    v1: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

/// @layout.struct name=test.box.Box size=4 align=4
/// @layout.field owner=test.box.Box index=0 name=weight offset=0 size=4 align=4
"#,
    );
}
