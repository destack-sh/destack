use crate::tests::TestSession;

/// An extension method lowers to a call taking a borrow of its target as the receiver.
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

    session.assert_mir_function(
        "main.ds",
        "test.main.Point.double",
        r#"
type test.main.Point {
    x: int32;
}

function test.main.Point.double<'a>(v0: ref<test.main.Point, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Point, borrowed, 'a, readonly>

entry(v0: ref<test.main.Point, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Point, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    v3: ref<test.main.Point, borrowed, 'a, readonly> = load l0
    v4: int32 = load (*v3).0
    v5: int32 = add v2, v4
    return v5
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.measure", r#"
type test.main.Point {
    x: int32;
}

function test.main.measure(): int32 {
    local l0: test.main.Point

entry:
    v0: int32 = 3
    v1: test.main.Point = aggregate (v0)
    store l0, v1
    v2: ref<test.main.Point, borrowed, 'frame, readonly> = address l0
    v3: int32 = call test.main.Point.double(v2): (ref<test.main.Point, borrowed, 'frame, readonly>) => int32
    return v3
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#);
}

/// A static call on a generic extension lowers to the instance its argument selects.
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

    session.assert_mir_function(
        "main.ds",
        "test.main.build",
        r#"
type test.main.Box<T> {
    value: T;
}

function test.main.build(v0: int32): test.main.Box<int32> {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: test.main.Box<int32> = call test.main.Box.of<int32>(v1): (int32) => test.main.Box<int32>
    return v2
}

/// @layout.struct name=test.main.Box<int32> size=4 align=4
/// @layout.field owner=test.main.Box<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Box.of<int32>",
        r#"
type test.main.Box<T> {
    value: T;
}

shared function test.main.Box.of<int32>(v0: int32): test.main.Box<int32>;

/// @layout.struct name=test.main.Box<int32> size=4 align=4
/// @layout.field owner=test.main.Box<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );
}

/// A generic static call lowers to the instance its expected result type selects.
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

    session.assert_mir_function(
        "main.ds",
        "test.main.build",
        r#"
type test.main.Box<T> {
    value: T;
}

function test.main.build(): test.main.Box<int32> {
entry:
    v0: int32 = 3
    v1: test.main.Box<int32> = call test.main.Box.of<int32>(v0): (int32) => test.main.Box<int32>
    return v1
}

/// @layout.struct name=test.main.Box<int32> size=4 align=4
/// @layout.field owner=test.main.Box<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Box.of<int32>",
        r#"
type test.main.Box<T> {
    value: T;
}

shared function test.main.Box.of<int32>(v0: int32): test.main.Box<int32>;

/// @layout.struct name=test.main.Box<int32> size=4 align=4
/// @layout.field owner=test.main.Box<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );
}

/// Lower a static call selected through a type literal receiver.
#[test]
fn test_lower_static_call_through_type_literal_receiver() {
    let session = TestSession::single(
        r#"
extension of int32 {
    static top(): int32 {
        return 5;
    }
}

function greatest(): int32 {
    return int32.top();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Number.top",
        r#"
function test.main.Number.top(): int32 {
entry:
    v0: int32 = 5
    return v0
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.greatest",
        r#"
function test.main.greatest(): int32 {
entry:
    v0: int32 = call test.main.Number.top(): () => int32
    return v0
}
"#,
    );
}
