use crate::tests::{DirRows, TestSession};

#[test]
fn test_union_constructor_selects_variant_member() {
    let session = TestSession::single(
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
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle

    draw(): void {}
    /// @type.symbol symbol=Rectangle.draw type=(this: Rectangle) => void
}

struct Circle {
/// @type.symbol symbol=Circle type=Circle

    draw(): void {}
    /// @type.symbol symbol=Circle.draw type=(this: Circle) => void
}

let shape: Rectangle | Circle = Rectangle {};
/// @type.symbol symbol=shape type=Rectangle | Circle

shape.draw();
/// @resolution.name source=shape target=shape
/// @resolution.member source=shape.draw receiver=Rectangle | Circle kind=select targets=[Rectangle.draw, Circle.draw]
/// @resolution.call source="shape.draw()" parameters=() return=void kind=select targets=[Rectangle.draw, Circle.draw]

"#);
}

#[test]
fn test_union_parameters_remain_transparent() {
    let session = TestSession::single(
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
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle

    draw(): void {}
    /// @type.symbol symbol=Rectangle.draw type=(this: Rectangle) => void
}

struct Circle {
/// @type.symbol symbol=Circle type=Circle

    draw(): void {}
    /// @type.symbol symbol=Circle.draw type=(this: Circle) => void
}

type Shape = Rectangle | Circle;
/// @type.symbol symbol=Shape type=Rectangle | Circle

function draw(shape: Shape): void {
/// @type.symbol symbol=draw type=<draw.T0: Shape>(draw.T0) => void
/// @generic.slot symbol=draw.T0 index=0 kind=type constraint=Shape

    shape.draw();
    /// @resolution.name source=shape target=shape
    /// @resolution.member source=shape.draw receiver=draw.T0 kind=select targets=[Rectangle.draw, Circle.draw]
    /// @resolution.call source="shape.draw()" parameters=() return=void kind=select targets=[Rectangle.draw, Circle.draw]
}
"#);
}
