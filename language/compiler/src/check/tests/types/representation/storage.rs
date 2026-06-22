use crate::tests::{DirRows, TestSession};

#[test]
fn test_type_alias_field_reifies_exact_storage() {
    let session = TestSession::single(
        r#"
type Point = {
    x: int32;
    y: int32;
};

struct Rectangle {
    start: Point;
}

const rectangle = Rectangle {
    start: { x: 0, y: 0 },
};

rectangle.start satisfies Point;
rectangle.start.x satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Point = {
    x: int32;
    y: int32;
};

struct Rectangle {
    start: Point;
}

const rectangle: Rectangle = Rectangle {
    start: { x: 0, y: 0 },
};

rectangle.start satisfies Point;
rectangle.start.x satisfies int32;

=== checked ===
type Point = {
/// @type.symbol symbol=Point source="type Point = {\n    x: int32;\n    y: int32;\n}" type={ x: int32; y: int32 }
/// @definition.type symbol=Point source="type Point = {\n    x: int32;\n    y: int32;\n}" value={ x: int32; y: int32 }

    x: int32;
    y: int32;
};

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.start source="start: Point" type={ x: int32; y: int32 }

    start: Point;
    /// @type.symbol symbol=Rectangle.start source="start: Point" type={ x: int32; y: int32 }
    /// @resolution.name source=Point target=Point

}

const rectangle = Rectangle {
/// @type.symbol symbol=rectangle type=Rectangle
/// @resolution.name source=Rectangle target=Rectangle

    start: { x: 0, y: 0 },
};

rectangle.start satisfies Point;
/// @resolution.name source=rectangle target=rectangle
/// @resolution.member source=rectangle.start receiver=Rectangle kind=symbol target=Rectangle.start
/// @resolution.name source=Point target=Point

rectangle.start.x satisfies int32;
/// @resolution.name source=rectangle target=rectangle
/// @resolution.member source=rectangle.start receiver=Rectangle kind=symbol target=Rectangle.start
/// @resolution.member source=rectangle.start.x receiver={ x: int32; y: int32 } kind=field key=x
"#,
    );
}

#[test]
fn test_type_alias_field_rejects_extra_property() {
    let session = TestSession::single(
        r#"
type Point = {
    x: int32;
    y: int32;
};

struct Rectangle {
    start: Point;
}

const rectangle = Rectangle {
    start: { x: 0, y: 0, z: 0 },
};
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Point = {
    x: int32;
    y: int32;
};

struct Rectangle {
    start: Point;
}

const rectangle: Rectangle = Rectangle {
    start: { x: 0, y: 0, z: 0 },
};

=== checked ===
type Point = {
/// @type.symbol symbol=Point source="type Point = {\n    x: int32;\n    y: int32;\n}" type={ x: int32; y: int32 }
/// @definition.type symbol=Point source="type Point = {\n    x: int32;\n    y: int32;\n}" value={ x: int32; y: int32 }

    x: int32;
    y: int32;
};

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.start source="start: Point" type={ x: int32; y: int32 }

    start: Point;
    /// @type.symbol symbol=Rectangle.start source="start: Point" type={ x: int32; y: int32 }
    /// @resolution.name source=Point target=Point

}

const rectangle = Rectangle {
/// @type.symbol symbol=rectangle type=Rectangle
/// @resolution.name source=Rectangle target=Rectangle

    start: { x: 0, y: 0, z: 0 },
};
"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'z' in object literal for type '{ x: int32; y: int32 }'"
/// @diagnostic.label line=11 column=27 source="    start: { x: 0, y: 0, z: 0 },"
"#,
    );
}

#[test]
fn test_interface_field_induces_hidden_generic_parameters() {
    let session = TestSession::single(
        r#"
interface PointLike {
    x: int32;
    y: int32;
}

struct Rectangle {
    start: PointLike;
    end: PointLike;
}

struct Point implements PointLike {
    x: int32;
    y: int32;
}

struct Offset implements PointLike {
    x: int32;
    y: int32;
}

const rectangle = Rectangle {
    start: Point { x: 0, y: 0 },
    end: Offset { x: 1, y: 1 },
};

rectangle.start satisfies Point;
rectangle.end satisfies Offset;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface PointLike {
    x: int32;
    y: int32;
}

struct Rectangle<T0: PointLike, T1: PointLike> {
    start: T0;
    end: T1;
}

struct Point implements PointLike {
    x: int32;
    y: int32;
}

struct Offset implements PointLike {
    x: int32;
    y: int32;
}

const rectangle: Rectangle<Point, Offset> = Rectangle<Point, Offset> {
    start: Point { x: 0, y: 0 },
    end: Offset { x: 1, y: 1 },
};

rectangle.start satisfies Point;
rectangle.end satisfies Offset;

=== checked ===
interface PointLike {
/// @type.symbol symbol=PointLike type=PointLike
/// @definition.interface symbol=PointLike
/// @definition.field symbol=PointLike.x source="x: int32" key=x type=int32
/// @definition.field symbol=PointLike.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=PointLike.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=PointLike.y source="y: int32" type=int32

}

struct Rectangle {
/// @generic.template symbol=Rectangle parameters=(T0: PointLike, T1: PointLike)
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle template=LocalGenericTemplateId(0)
/// @definition.field symbol=Rectangle.start source="start: PointLike" type=T0
/// @definition.field symbol=Rectangle.end source="end: PointLike" type=T1

    start: PointLike;
    /// @type.symbol symbol=Rectangle.start source="start: PointLike" type=T0
    /// @resolution.name source=PointLike target=PointLike

    end: PointLike;
    /// @type.symbol symbol=Rectangle.end source="end: PointLike" type=T1
    /// @resolution.name source=PointLike target=PointLike

}

struct Point implements PointLike {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32
/// @resolution.name source=PointLike target=PointLike

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

struct Offset implements PointLike {
/// @type.symbol symbol=Offset type=Offset
/// @definition.struct symbol=Offset
/// @definition.field symbol=Offset.x source="x: int32" key=x type=int32
/// @definition.field symbol=Offset.y source="y: int32" key=y type=int32
/// @resolution.name source=PointLike target=PointLike

    x: int32;
    /// @type.symbol symbol=Offset.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Offset.y source="y: int32" type=int32

}

const rectangle = Rectangle {
/// @type.symbol symbol=rectangle type=Rectangle<Point, Offset>
/// @resolution.name source=Rectangle target=Rectangle
/// @generic.instance source=Rectangle id="Rectangle<Point, Offset>"

    start: Point { x: 0, y: 0 },
    /// @resolution.name source=Point target=Point

    end: Offset { x: 1, y: 1 },
    /// @resolution.name source=Offset target=Offset

};

rectangle.start satisfies Point;
/// @resolution.name source=rectangle target=rectangle
/// @resolution.member source=rectangle.start receiver=Rectangle<Point, Offset> kind=symbol target=Rectangle.start instance="Rectangle<Point, Offset>"
/// @resolution.name source=Point target=Point

rectangle.end satisfies Offset;
/// @resolution.name source=rectangle target=rectangle
/// @resolution.member source=rectangle.end receiver=Rectangle<Point, Offset> kind=symbol target=Rectangle.end instance="Rectangle<Point, Offset>"
/// @resolution.name source=Offset target=Offset
/// @generic.instance id="Rectangle<Point, Offset>" template=Rectangle arguments=(Point, Offset)
"#,
    );
}

#[test]
fn test_array_element_type_reifies_alias_storage() {
    let session = TestSession::single(
        r#"
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

const shapes: Array<Shape> = [
    Circle { radius: 1.0 },
    Rectangle { width: 1.0, height: 1.0 },
];

const first = shapes[0];
first satisfies Shape;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

const shapes: Array<Shape> = [
    Circle { radius: 1.0 },
    Rectangle { width: 1.0, height: 1.0 },
];

const first: Shape = shapes[0];
first satisfies Shape;

=== checked ===
struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.field symbol=Circle.radius source="radius: float64" key=radius type=float64

    radius: float64;
    /// @type.symbol symbol=Circle.radius source="radius: float64" type=float64

}

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.width source="width: float64" key=width type=float64
/// @definition.field symbol=Rectangle.height source="height: float64" key=height type=float64

    width: float64;
    /// @type.symbol symbol=Rectangle.width source="width: float64" type=float64

    height: float64;
    /// @type.symbol symbol=Rectangle.height source="height: float64" type=float64

}

type Shape = Circle | Rectangle;
/// @type.symbol symbol=Shape source="type Shape = Circle | Rectangle" type=Circle | Rectangle
/// @definition.type symbol=Shape source="type Shape = Circle | Rectangle" value=Circle | Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

const shapes: Array<Shape> = [
/// @type.symbol symbol=shapes source=shapes type=Array<Shape>
/// @resolution.name source=Array target=collections.array.Array
/// @resolution.name source=Shape target=Shape

    Circle { radius: 1.0 },
    /// @resolution.name source=Circle target=Circle

    Rectangle { width: 1.0, height: 1.0 },
    /// @resolution.name source=Rectangle target=Rectangle

];

const first = shapes[0];
/// @type.symbol symbol=first type=Shape
/// @resolution.name source=shapes target=shapes
/// @resolution.call source=shapes[0] parameters=(usize) return=Shape kind=symbol target=collections.array.index#9 receiver=Array<Shape>

first satisfies Shape;
/// @resolution.name source=first target=first
/// @resolution.name source=Shape target=Shape
"#,
    );
}

#[test]
fn test_literal_union_field_reifies_storage() {
    let session = TestSession::single(
        r#"
type Mode = "active" | "paused";

struct Player {
    mode: Mode;
}

const player = Player {
    mode: "active",
};

player.mode satisfies Mode;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Mode = "active" | "paused";

struct Player {
    mode: Mode;
}

const player: Player = Player {
    mode: "active",
};

player.mode satisfies Mode;

=== checked ===
type Mode = "active" | "paused";
/// @type.symbol symbol=Mode source="type Mode = \"active\" | \"paused\"" type="active" | "paused"
/// @definition.type symbol=Mode source="type Mode = \"active\" | \"paused\"" value="active" | "paused"

struct Player {
/// @type.symbol symbol=Player type=Player
/// @definition.struct symbol=Player
/// @definition.field symbol=Player.mode source="mode: Mode" type=Mode

    mode: Mode;
    /// @type.symbol symbol=Player.mode source="mode: Mode" type=Mode
    /// @resolution.name source=Mode target=Mode

}

const player = Player {
/// @type.symbol symbol=player source=player type=Player
/// @resolution.name source=Player target=Player

    mode: "active",
    /// @type.node source="\"active\"" type="active"

};

player.mode satisfies Mode;
/// @resolution.name source=player target=player
/// @resolution.member source=player.mode receiver=Player kind=symbol target=Player.mode
/// @resolution.name source=Mode target=Mode
"#,
    );
}

#[test]
fn test_struct_and_class_fields_reify_alias_storage() {
    let session = TestSession::single(
        r#"
type Point = { x: int32; y: int32 };

struct Segment {
    start: Point;
}

class Marker {
    position: Point;
}

declare const segment: Segment;
declare const marker: Marker;

segment.start satisfies Point;
marker.position satisfies Point;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Point = { x: int32; y: int32 };

struct Segment {
    start: Point;
}

class Marker {
    position: Point;
}

declare const segment: Segment;
declare const marker: Marker;

segment.start satisfies Point;
marker.position satisfies Point;

=== checked ===
type Point = { x: int32; y: int32 };
/// @type.symbol symbol=Point source="type Point = { x: int32; y: int32 }" type={ x: int32; y: int32 }
/// @definition.type symbol=Point source="type Point = { x: int32; y: int32 }" value={ x: int32; y: int32 }

struct Segment {
/// @type.symbol symbol=Segment type=Segment
/// @definition.struct symbol=Segment
/// @definition.field symbol=Segment.start source="start: Point" type={ x: int32; y: int32 }

    start: Point;
    /// @type.symbol symbol=Segment.start source="start: Point" type={ x: int32; y: int32 }
    /// @resolution.name source=Point target=Point

}

class Marker {
/// @type.symbol symbol=Marker type=Marker
/// @definition.class symbol=Marker
/// @definition.field symbol=Marker.position source="position: Point" type={ x: int32; y: int32 }

    position: Point;
    /// @type.symbol symbol=Marker.position source="position: Point" type={ x: int32; y: int32 }
    /// @resolution.name source=Point target=Point

}

declare const segment: Segment;
/// @type.symbol symbol=segment source=segment type=Segment
/// @resolution.name source=Segment target=Segment

declare const marker: Marker;
/// @type.symbol symbol=marker source=marker type=Marker
/// @resolution.name source=Marker target=Marker

segment.start satisfies Point;
/// @resolution.name source=segment target=segment
/// @resolution.member source=segment.start receiver=Segment kind=symbol target=Segment.start
/// @resolution.name source=Point target=Point

marker.position satisfies Point;
/// @resolution.name source=marker target=marker
/// @resolution.member source=marker.position receiver=Marker kind=symbol target=Marker.position
/// @resolution.name source=Point target=Point
"#,
    );
}
