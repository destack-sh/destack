use super::super::snapshot::assert_check_snapshot;

#[test]
fn test_check_records_stored_union_selection() {
    assert_check_snapshot(
        r#"
struct Rectangle {
    draw(): void {}
}

struct Circle {
    draw(): void {}
}

let shape: Rectangle | Circle = Rectangle {};
shape.draw();
"#,
        r#"
struct Rectangle {
/// @type.symbol key=Rectangle value=Rectangle

    draw(): void {}
    /// @type.symbol key=Rectangle.draw value=(this: Rectangle) => void
}

struct Circle {
/// @type.symbol key=Circle value=Circle

    draw(): void {}
    /// @type.symbol key=Circle.draw value=(this: Circle) => void
}

let shape: Rectangle | Circle = Rectangle {};
/// @type.symbol key=shape value=Rectangle | Circle
/// @layout.type type=Rectangle | Circle layout=layout0 shape=variant

shape.draw();
/// @resolution.name source=shape target=shape
/// @resolution.member source=shape.draw receiver=Rectangle | Circle kind=select targets=[Rectangle.draw, Circle.draw]
/// @resolution.call source="shape.draw()" parameters=[] return=void kind=select targets=[Rectangle.draw, Circle.draw]

/// @layout.entry layout=layout0 shape=variant
/// @layout.summary layouts=1 types=1
/// @type.summary types=4 nodes=1 symbols=5
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=1 calls=1
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
"#,
    );
}

#[test]
fn test_check_keeps_union_parameters_transparent() {
    assert_check_snapshot(
        r#"
struct Rectangle {
    draw(): void {}
}

struct Circle {
    draw(): void {}
}

type Shape = Rectangle | Circle;

function draw(shape: Shape): void {
    shape.draw();
}
"#,
        r#"
struct Rectangle {
/// @type.symbol key=Rectangle value=Rectangle

    draw(): void {}
    /// @type.symbol key=Rectangle.draw value=(this: Rectangle) => void
}

struct Circle {
/// @type.symbol key=Circle value=Circle

    draw(): void {}
    /// @type.symbol key=Circle.draw value=(this: Circle) => void
}

type Shape = Rectangle | Circle;
/// @type.symbol key=Shape value=Rectangle | Circle

function draw(shape: Shape): void {
/// @generic.parameters key=draw parameters=[draw.T0]
/// @type.symbol key=draw value=<draw.T0: Shape>(draw.T0) => void
/// @generic.parameter key=draw.T0 space=type constraint=Shape

    shape.draw();
    /// @resolution.name source=shape target=shape
    /// @resolution.member source=shape.draw receiver=draw.T0 kind=direct target=Shape.draw
    /// @resolution.call source="shape.draw()" parameters=[] return=void kind=direct target=Shape.draw
}

/// @type.summary types=6 nodes=1 symbols=8
/// @generic.summary parameters=1 lists=1
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=1 calls=1
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}
