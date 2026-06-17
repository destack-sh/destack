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

let point: Placed<Point, "local"> = Point { x: 1, y: 2 };
point satisfies WithSpace<Point, "local">;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32
}

let point: local Point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Placed<Point, "local">
/// @resolution.name source=Point target=Point
/// @resolution.name source=Point target=Point

point satisfies WithSpace<Point, "local">;
/// @resolution.name source=point target=point
/// @resolution.name source=WithSpace target=memory.WithSpace
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

let point: Placed<Point, "shared"> = Point { x: 1, y: 2 };
point satisfies Placed<Point, "shared">;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32
}

let point: shared Point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Placed<Point, "shared">
/// @resolution.name source=Point target=Point
/// @resolution.name source=Point target=Point

point satisfies shared Point;
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

function kernel<comptime L0: Lifetime>(data: Borrowed<Placed<Point, "shared">, L0, "mutable">): int32 {
    return data.x;
}

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32
}

function kernel(data: shared &Point): int32 {
/// @generic.template symbol=kernel parameters=[comptime L0: Lifetime origin=induced.form]
/// @type.symbol symbol=kernel type=(Borrowed<Placed<Point, "shared">, kernel.L0, "mutable">) => int32
/// @type.symbol symbol=data source="data: shared &Point" type=Borrowed<Placed<Point, "shared">, kernel.L0, "mutable">
/// @resolution.name source=Point target=Point

    return data.x;
    /// @resolution.name source=data target=data
    /// @resolution.member source=data.x receiver=Borrowed<Placed<Point, "shared">, kernel.L0, "mutable"> kind=symbol target=Point.x
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

struct Request<T> {
    header: Header;
    body: T;
}

declare let request: Placed<Request<Payload>, "shared">;

request.header satisfies Placed<Header, "shared">;
request.body satisfies Placed<Payload, "shared">;

=== checked ===
struct Header {
/// @type.symbol symbol=Header type=Header
/// @definition.struct symbol=Header

    id: int32;
    /// @type.symbol symbol=Header.id source="id: int32" type=int32
}

struct Payload {
/// @type.symbol symbol=Payload type=Payload
/// @definition.struct symbol=Payload

    value: int32;
    /// @type.symbol symbol=Payload.value source="value: int32" type=int32
}

struct Request<T> {
/// @generic.template symbol=Request parameters=[T]
/// @type.symbol symbol=Request type=Request<T>
/// @definition.struct symbol=Request

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
/// @resolution.name source=request target=request
/// @resolution.member source=request.header receiver=Placed<Request<Payload>, "shared"> kind=symbol target=Request.header
/// @resolution.name source=Header target=Header

request.body satisfies shared Payload;
/// @resolution.name source=request target=request
/// @resolution.member source=request.body receiver=Placed<Request<Payload>, "shared"> kind=symbol target=Request.body
/// @resolution.name source=Payload target=Payload
"#,
    );
}
