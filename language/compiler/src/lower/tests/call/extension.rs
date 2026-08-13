use crate::tests::TestSession;

#[test]
fn test_lower_extension_method_through_its_target_receiver() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

extension of Point {
    double(): int32 {
        return this.x + this.x;
    }
}

function measure(): int32 {
    let point = Point { x: 3 };
    return point.double();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
}

function test.main.Point.double<'a>(v0: ref<Point, borrowed, 'a, exclusive>): int32 {
entry(v0: ref<Point, borrowed, 'a, exclusive>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, exclusive> = field.address v0, 0
    v4: int32 = load v3
    v5: int32 = int.add v2, v4
    return v5
}

function test.main.measure(): int32 {
    local l0: Point

entry:
    v0: int32 = 3
    v1: Point = aggregate (v0)
    local.set l0, v1
    v2: ref<Point, borrowed, 'frame, exclusive> = local.address l0
    v3: int32 = call test.main.Point.double(v2): <'a>(ref<Point, borrowed, 'a, exclusive>) => int32
    return v3
}
/// @layout.struct name=Point size=4 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_a_static_call_scoped_by_its_generic_extension() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    value: T;
}

extension<T> of Box<T> {
    static of(value: T): Box<T> {
        return Box<T> { value };
    }
}

function build(value: int32): Box<int32> {
    return Box.of(value);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Box<int32> {
    value: int32;
}

function test.main.build(v0: int32): Box<int32> {
entry(v0: int32):
    v1: Box<int32> = call test.main.of<int32>(v0): (int32) => Box<int32>
    return v1
}

function test.main.of<int32>(v0: int32): Box<int32> {
entry(v0: int32):
    v1: Box<int32> = aggregate (v0)
    return v1
}
/// @layout.struct name=Box<int32> size=4 align=4
/// @layout.field owner=Box<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_a_generic_static_call_instantiated_by_its_contextual_result() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    value: T;
}

extension<T> of Box<T> {
    static of(value: T): Box<T> {
        return Box<T> { value };
    }
}

function build(): Box<int32> {
    return Box.of(3);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Box<int32> {
    value: int32;
}

function test.main.build(): Box<int32> {
entry:
    v0: int32 = 3
    v1: Box<int32> = call test.main.of<int32>(v0): (int32) => Box<int32>
    return v1
}

function test.main.of<int32>(v0: int32): Box<int32> {
entry(v0: int32):
    v1: Box<int32> = aggregate (v0)
    return v1
}
/// @layout.struct name=Box<int32> size=4 align=4
/// @layout.field owner=Box<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );
}
