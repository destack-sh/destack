use crate::tests::{DirRows, TestSession};

#[test]
fn test_union_member_call_selects_each_variant_method() {
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
    /// @type.symbol symbol=Rectangle.draw type=(this: Rectangle) => ()

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle

    draw(): void {}
    /// @type.symbol symbol=Circle.draw type=(this: Circle) => ()

}

let shape: Rectangle | Circle = Rectangle {};
/// @type.symbol symbol=shape type=Rectangle | Circle
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

shape.draw();
/// @resolution.name source=shape target=shape
/// @resolution.member source=shape.draw receiver=Rectangle | Circle kind=select targets=[Rectangle.draw, Circle.draw]
/// @resolution.call source=shape.draw() parameters=() return=() kind=select targets=[Rectangle.draw, Circle.draw]

"#);
}

#[test]
fn test_union_alias_parameter_induces_constrained_generic() {
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
    /// @type.symbol symbol=Rectangle.draw type=(this: Rectangle) => ()

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle

    draw(): void {}
    /// @type.symbol symbol=Circle.draw type=(this: Circle) => ()

}

type Shape = Rectangle | Circle;
/// @type.symbol symbol=Shape type=Rectangle | Circle
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle

function draw(shape: Shape): void {
/// @generic.slot key=draw.T0 index=0 kind=type constraint=Shape
/// @type.symbol symbol=draw type=<draw.T0: Shape>(draw.T0) => ()
/// @type.symbol symbol=shape type=draw.T0
/// @resolution.name source=Shape target=Shape

    shape.draw();
    /// @resolution.name source=shape target=shape
    /// @resolution.member source=shape.draw receiver=draw.T0 kind=select targets=[Rectangle.draw, Circle.draw]
    /// @resolution.call source=shape.draw() parameters=() return=() kind=select targets=[Rectangle.draw, Circle.draw]

}
"#);
}
