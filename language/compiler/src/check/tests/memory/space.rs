use crate::tests::{DirRows, TestSession};

#[test]
fn test_local_annotation_places_value() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

let point: local Point = Point { x: 1, y: 2 };
point satisfies WithSpace<Point, "local">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

let point: local Point = Point { x: 1, y: 2 };
point satisfies WithSpace<Point, "local">;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

let point: local Point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Placed<Point, "local">
/// @resolution.name source=Point target=Point
/// @type.node source="Point { x: 1, y: 2 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
/// @type.node source=2 type=2

point satisfies WithSpace<Point, "local">;
/// @type.node source="point satisfies WithSpace<Point, \"local\">" type=Placed<Point, "local">
/// @type.node source=point type=Placed<Point, "local">
/// @resolution.name source=point target=point
/// @resolution.name source=WithSpace target=memory.type.WithSpace
/// @resolution.name source=Point target=Point
"#,
    );
}

#[test]
fn test_shared_annotation_places_value() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

let point: shared Point = Point { x: 1, y: 2 };
point satisfies shared Point;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

let point: shared Point = Point { x: 1, y: 2 };
point satisfies shared Point;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

let point: shared Point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Placed<Point, "shared">
/// @resolution.name source=Point target=Point
/// @type.node source="Point { x: 1, y: 2 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
/// @type.node source=2 type=2

point satisfies shared Point;
/// @type.node source="point satisfies shared Point" type=Placed<Point, "shared">
/// @type.node source=point type=Placed<Point, "shared">
/// @resolution.name source=point target=point
/// @resolution.name source=Point target=Point
"#,
    );
}

#[test]
fn test_shared_borrow_reads_shared_storage() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function kernel(data: shared &Point): int32 {
    return data.x;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

function kernel<comptime L0: Lifetime>(data: shared Borrowed<Point, L0, "mutable">): int32 {
    return data.x;
}

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

function kernel(data: shared &Point): int32 {
/// @generic.template symbol=kernel parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=kernel type=<comptime kernel.L0: Lifetime>(Placed<Borrowed<Point, kernel.L0, "mutable">, "shared">) => int32
/// @type.symbol symbol=kernel.data source="data: shared &Point" type=Placed<Borrowed<Point, kernel.L0, "mutable">, "shared">
/// @resolution.name source=Point target=Point

    return data.x;
    /// @type.node source=data type=Placed<Borrowed<Point, kernel.L0, "mutable">, "shared">
    /// @type.node source=data.x type=Placed<int32, "shared">
    /// @resolution.name source=data target=kernel.data
    /// @resolution.member source=data.x receiver=Placed<Borrowed<Point, kernel.L0, "mutable">, "shared"> kind=symbol target=Point.x

}
"#,
    );
}

#[test]
fn test_shared_aggregate_fields_inherit_shared_space() {
    let session = TestSession::single(
        r#"
struct Header {
    id: int32;
}

struct Payload {
    value: int32;
}

struct Request<T> {
    header: Header;
    body: T;
}

declare let request: shared Request<Payload>;

request.header satisfies shared Header;
request.body satisfies shared Payload;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Header {
    id: int32;
}

struct Payload {
    value: int32;
}

struct Request<out T> {
    header: Header;
    body: T;
}

declare let request: shared Request<Payload>;

request.header satisfies shared Header;
request.body satisfies shared Payload;

=== checked ===
struct Header {
/// @type.symbol symbol=Header type=Header
/// @definition.struct symbol=Header
/// @definition.field symbol=Header.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Header.id source="id: int32" type=int32

}

struct Payload {
/// @type.symbol symbol=Payload type=Payload
/// @definition.struct symbol=Payload
/// @definition.field symbol=Payload.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Payload.value source="value: int32" type=int32

}

struct Request<T> {
/// @generic.template symbol=Request parameters=(out T)
/// @type.symbol symbol=Request type=Request
/// @definition.struct symbol=Request template=(out T)
/// @definition.field symbol=Request.body source="body: T" key=body type=T
/// @definition.field symbol=Request.header source="header: Header" key=header type=Header
/// @type.symbol symbol=Request.T source=T type=T

    header: Header;
    /// @type.symbol symbol=Request.header source="header: Header" type=Header
    /// @resolution.name source=Header target=Header

    body: T;
    /// @type.symbol symbol=Request.body source="body: T" type=T
    /// @resolution.name source=T target=Request.T

}

declare let request: shared Request<Payload>;
/// @type.symbol symbol=request source=request type=Placed<Request<Payload>, "shared">
/// @resolution.name source=Request target=Request
/// @resolution.name source=Payload target=Payload

request.header satisfies shared Header;
/// @type.node source="request.header satisfies shared Header" type=Placed<Header, "shared">
/// @type.node source=request type=Placed<Request<Payload>, "shared">
/// @type.node source=request.header type=Placed<Header, "shared">
/// @resolution.name source=request target=request
/// @resolution.member source=request.header receiver=Placed<Request<Payload>, "shared"> kind=symbol target=Request.header
/// @generic.instance source=request id=Request<Payload>
/// @resolution.name source=Header target=Header

request.body satisfies shared Payload;
/// @type.node source="request.body satisfies shared Payload" type=Placed<Payload, "shared">
/// @type.node source=request type=Placed<Request<Payload>, "shared">
/// @type.node source=request.body type=Placed<Payload, "shared">
/// @resolution.name source=request target=request
/// @resolution.member source=request.body receiver=Placed<Request<Payload>, "shared"> kind=symbol target=Request.body
/// @generic.instance source=request id=Request<Payload>
/// @resolution.name source=Payload target=Payload

/// @generic.instance id=Request<Payload> template=Request arguments=(Payload)
"#,
    );
}

#[test]
fn test_handles_do_not_cross_spaces() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

declare const remote: shared ^Point;
const nearby: local ^Point = remote;

declare const far: shared int32;
const near: int32 = far;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

declare const remote: shared ^Point;
const nearby: local ^Point = remote;

declare const far: shared int32;
const near: int32 = far;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

declare const remote: shared ^Point;
/// @type.symbol symbol=remote source=remote type=Placed<Owned<Point>, "shared"> reduced=Placed<Point, "shared">
/// @resolution.name source=Point target=Point

const nearby: local ^Point = remote;
/// @type.symbol symbol=nearby source=nearby type=Placed<Owned<Point>, "local"> reduced=Placed<Point, "local">
/// @resolution.name source=Point target=Point
/// @resolution.name source=remote target=remote

declare const far: shared int32;
/// @type.symbol symbol=far source=far type=Placed<int32, "shared">

const near: int32 = far;
/// @type.symbol symbol=near source=near type=int32
/// @resolution.name source=far target=far
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'shared ^Point' is not assignable to type 'local ^Point'"
/// @diagnostic.label line=7 column=30 span="remote" line_source="const nearby: local ^Point = remote;"
"#,
    );
}
