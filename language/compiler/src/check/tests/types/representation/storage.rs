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
    start: { x: int32; y: int32 };
}

const rectangle: Rectangle = Rectangle {
    start: { x: 0, y: 0 },
};

rectangle.start satisfies Point;
rectangle.start.x satisfies int32;

=== checked ===
type Point = {
/// @type.symbol symbol=Point type={ x: int32; y: int32 }
/// @definition.type symbol=Point value={ x: int32; y: int32 }

    x: int32;
    y: int32;
};

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.start source="start: Point" key=start type={ x: int32; y: int32 }

    start: Point;
    /// @type.symbol symbol=Rectangle.start source="start: Point" type={ x: int32; y: int32 }
    /// @resolution.name source=Point target=Point

}

const rectangle = Rectangle {
/// @type.symbol symbol=rectangle source=rectangle type=Rectangle
/// @resolution.pattern source=rectangle kind=binding target=rectangle
/// @resolution.name source=Rectangle target=Rectangle

    start: { x: 0, y: 0 },
};

rectangle.start satisfies Point;
/// @resolution.name source=rectangle target=rectangle
/// @resolution.member source=rectangle.start receiver=Rectangle type=Point kind=field target_receiver=Rectangle key=start target=Rectangle.start target_type=Point
/// @resolution.place source=rectangle placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle root=rectangle
/// @resolution.place source=rectangle.start placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle.start root=rectangle keys=[start]
/// @resolution.name source=Point target=Point

rectangle.start.x satisfies int32;
/// @resolution.name source=rectangle target=rectangle
/// @resolution.member source=rectangle.start receiver=Rectangle type=Point kind=field target_receiver=Rectangle key=start target=Rectangle.start target_type=Point
/// @resolution.member source=rectangle.start.x receiver=Point type=int32 kind=field target_receiver=Point key=x target_type=int32
/// @resolution.place source=rectangle placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle root=rectangle
/// @resolution.place source=rectangle.start placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle.start root=rectangle keys=[start]
/// @resolution.place source=rectangle.start.x placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle.start.x root=rectangle keys=[start, x]
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
    start: { x: int32; y: int32 };
}

const rectangle: Rectangle = Rectangle {
    start: { x: 0, y: 0, z: 0 },
};

=== checked ===
type Point = {
/// @type.symbol symbol=Point type={ x: int32; y: int32 }
/// @definition.type symbol=Point value={ x: int32; y: int32 }

    x: int32;
    y: int32;
};

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.start source="start: Point" key=start type={ x: int32; y: int32 }

    start: Point;
    /// @type.symbol symbol=Rectangle.start source="start: Point" type={ x: int32; y: int32 }
    /// @resolution.name source=Point target=Point

}

const rectangle = Rectangle {
/// @type.symbol symbol=rectangle source=rectangle type=Rectangle
/// @resolution.pattern source=rectangle kind=binding target=rectangle
/// @resolution.name source=Rectangle target=Rectangle

    start: { x: 0, y: 0, z: 0 },
};
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'z' in object literal for type '{ x: int32; y: int32 }'"
/// @diagnostic.label line=12 column=12 span="{ x: 0, y: 0, z: 0 }" line_source="start: { x: 0, y: 0, z: 0 },"
/// @diagnostic.related line=11 column=19 span="Rectangle {\n    start: { x: 0, y: 0, z: 0 },\n}" line_source="const rectangle = Rectangle {" message="expected due to the type of this target"
/// @diagnostic.note message="object literals may only specify known properties"
/// @diagnostic.note message="the mismatch is in field 'start'"
"#,
    );
}

#[test]
fn test_interface_fields_erase_to_dynamic_storage() {
    // interface-typed fields erase to Dynamic storage: only lifetimes induce generics
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

rectangle.start satisfies PointLike;
rectangle.end satisfies PointLike;
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

struct Rectangle {
    start: Dynamic<PointLike>;
    end: Dynamic<PointLike>;
}

struct Point implements PointLike {
    x: int32;
    y: int32;
}

struct Offset implements PointLike {
    x: int32;
    y: int32;
}

const rectangle: Rectangle = Rectangle {
    start: Point { x: 0, y: 0 } as Dynamic<PointLike>,
    end: Offset { x: 1, y: 1 } as Dynamic<PointLike>,
};

rectangle.start satisfies PointLike;
rectangle.end satisfies PointLike;

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
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.end source="end: PointLike" key=end type=Dynamic<PointLike>
/// @definition.field symbol=Rectangle.start source="start: PointLike" key=start type=Dynamic<PointLike>

    start: PointLike;
    /// @type.symbol symbol=Rectangle.start source="start: PointLike" type=Dynamic<PointLike>
    /// @resolution.name source=PointLike target=PointLike

    end: PointLike;
    /// @type.symbol symbol=Rectangle.end source="end: PointLike" type=Dynamic<PointLike>
    /// @resolution.name source=PointLike target=PointLike

}

struct Point implements PointLike {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.where symbol=Point source=PointLike relation=satisfies left=this right=PointLike
/// @definition.implements symbol=Point source=PointLike target=PointLike
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32
/// @definition.conformance symbol=Point member=Point.x requirement=PointLike.x
/// @definition.conformance symbol=Point member=Point.y requirement=PointLike.y
/// @resolution.name source=PointLike target=PointLike

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

struct Offset implements PointLike {
/// @type.symbol symbol=Offset type=Offset
/// @definition.struct symbol=Offset
/// @definition.where symbol=Offset source=PointLike relation=satisfies left=this right=PointLike
/// @definition.implements symbol=Offset source=PointLike target=PointLike
/// @definition.field symbol=Offset.x source="x: int32" key=x type=int32
/// @definition.field symbol=Offset.y source="y: int32" key=y type=int32
/// @definition.conformance symbol=Offset member=Offset.x requirement=PointLike.x
/// @definition.conformance symbol=Offset member=Offset.y requirement=PointLike.y
/// @resolution.name source=PointLike target=PointLike

    x: int32;
    /// @type.symbol symbol=Offset.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Offset.y source="y: int32" type=int32

}

const rectangle = Rectangle {
/// @type.symbol symbol=rectangle source=rectangle type=Rectangle
/// @resolution.pattern source=rectangle kind=binding target=rectangle
/// @resolution.name source=Rectangle target=Rectangle

    start: Point { x: 0, y: 0 },
    /// @resolution.name source=Point target=Point

    end: Offset { x: 1, y: 1 },
    /// @resolution.name source=Offset target=Offset

};

rectangle.start satisfies PointLike;
/// @resolution.name source=rectangle target=rectangle
/// @resolution.member source=rectangle.start receiver=Rectangle type=Dynamic<PointLike> kind=field target_receiver=Rectangle key=start target=Rectangle.start target_type=Dynamic<PointLike>
/// @resolution.place source=rectangle placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle root=rectangle
/// @resolution.place source=rectangle.start placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle.start root=rectangle keys=[start]
/// @resolution.name source=PointLike target=PointLike

rectangle.end satisfies PointLike;
/// @resolution.name source=rectangle target=rectangle
/// @resolution.member source=rectangle.end receiver=Rectangle type=Dynamic<PointLike> kind=field target_receiver=Rectangle key=end target=Rectangle.end target_type=Dynamic<PointLike>
/// @resolution.place source=rectangle placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle root=rectangle
/// @resolution.place source=rectangle.end placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rectangle.end root=rectangle keys=[end]
/// @resolution.name source=PointLike target=PointLike
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

const shapes: Shape[] = [
    Circle { radius: 1.0 } as Circle | Rectangle,
    Rectangle { width: 1.0, height: 1.0 } as Circle | Rectangle,
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
/// @definition.field symbol=Rectangle.height source="height: float64" key=height type=float64
/// @definition.field symbol=Rectangle.width source="width: float64" key=width type=float64

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
/// @resolution.pattern source=shapes kind=binding target=shapes
/// @resolution.name source=Array target=collections.array.Array
/// @resolution.name source=Shape target=Shape

    Circle { radius: 1.0 },
    /// @resolution.name source=Circle target=Circle

    Rectangle { width: 1.0, height: 1.0 },
    /// @resolution.name source=Rectangle target=Rectangle

];

const first = shapes[0];
/// @type.symbol symbol=first source=first type=Shape
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=shapes target=shapes
/// @resolution.place source=shapes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=shapes root=shapes
/// @resolution.access source=shapes[0] root=shapes keys=[0]
/// @resolution.subscript source=shapes[0] type=Shape kind=call target="collections.array.index#1(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'static Shape, \"exclusive\">)"
/// @generic.instance source=shapes[0] id="Array<Shape>.<extension#4>.index#1<\"exclusive\">"

first satisfies Shape;
/// @resolution.name source=first target=first
/// @resolution.place source=first placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=first root=first
/// @resolution.name source=Shape target=Shape

/// @generic.instance id="Array<Shape>.<extension#4>.index#1<\"exclusive\">" template=collections.array.index#1 arguments=(Shape, "exclusive")
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
    mode: "active" | "paused";
}

const player: Player = Player {
    mode: "active" as "active" | "paused",
};

player.mode satisfies Mode;

=== checked ===
type Mode = "active" | "paused";
/// @type.symbol symbol=Mode source="type Mode = \"active\" | \"paused\"" type="active" | "paused"
/// @definition.type symbol=Mode source="type Mode = \"active\" | \"paused\"" value="active" | "paused"

struct Player {
/// @type.symbol symbol=Player type=Player
/// @definition.struct symbol=Player
/// @definition.field symbol=Player.mode source="mode: Mode" key=mode type="active" | "paused"

    mode: Mode;
    /// @type.symbol symbol=Player.mode source="mode: Mode" type="active" | "paused"
    /// @resolution.name source=Mode target=Mode

}

const player = Player {
/// @type.symbol symbol=player source=player type=Player
/// @resolution.pattern source=player kind=binding target=player
/// @resolution.name source=Player target=Player

    mode: "active",
};

player.mode satisfies Mode;
/// @resolution.name source=player target=player
/// @resolution.member source=player.mode receiver=Player type=Mode kind=field target_receiver=Player key=mode target=Player.mode target_type=Mode
/// @resolution.place source=player placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=player root=player
/// @resolution.place source=player.mode placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=player.mode root=player keys=[mode]
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
    position!: Point;
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
    start: { x: int32; y: int32 };
}

class Marker {
    position!: { x: int32; y: int32 };
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
/// @definition.field symbol=Segment.start source="start: Point" key=start type={ x: int32; y: int32 }

    start: Point;
    /// @type.symbol symbol=Segment.start source="start: Point" type={ x: int32; y: int32 }
    /// @resolution.name source=Point target=Point

}

class Marker {
/// @type.symbol symbol=Marker type=Marker
/// @definition.class symbol=Marker
/// @definition.field symbol=Marker.position source="position!: Point" key=position type={ x: int32; y: int32 }

    position!: Point;
    /// @type.symbol symbol=Marker.position source="position!: Point" type={ x: int32; y: int32 }
    /// @resolution.name source=Point target=Point

}

declare const segment: Segment;
/// @type.symbol symbol=segment source=segment type=Segment
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=Segment target=Segment

declare const marker: Marker;
/// @type.symbol symbol=marker source=marker type=Marker
/// @resolution.pattern source=marker kind=binding target=marker
/// @resolution.name source=Marker target=Marker

segment.start satisfies Point;
/// @resolution.name source=segment target=segment
/// @resolution.member source=segment.start receiver=Segment type=Point kind=field target_receiver=Segment key=start target=Segment.start target_type=Point
/// @resolution.place source=segment placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=segment root=segment
/// @resolution.place source=segment.start placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=segment.start root=segment keys=[start]
/// @resolution.name source=Point target=Point

marker.position satisfies Point;
/// @resolution.name source=marker target=marker
/// @resolution.member source=marker.position receiver=Marker type=Point kind=field target_receiver=Marker key=position target=Marker.position target_type=Point
/// @resolution.place source=marker placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=marker root=marker
/// @resolution.place source=marker.position placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=marker.position root=marker keys=[position]
/// @resolution.name source=Point target=Point
"#,
    );
}
