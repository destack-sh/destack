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
=== annotated ===
struct Rectangle {
    draw(): void {}
}

struct Circle {
    draw(): void {}
}

let shape: Rectangle | Circle = Rectangle {};
shape.draw();

=== checked ===
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.method symbol=Rectangle.draw source="draw(): void {}" slot=draw type=(this: Rectangle) => void

    draw(): void {}
    /// @type.symbol symbol=Rectangle.draw source="draw(): void {}" type=(this: Rectangle) => void

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.method symbol=Circle.draw source="draw(): void {}" slot=draw type=(this: Circle) => void

    draw(): void {}
    /// @type.symbol symbol=Circle.draw source="draw(): void {}" type=(this: Circle) => void

}

let shape: Rectangle | Circle = Rectangle {};
/// @type.symbol symbol=shape source=shape type=Rectangle | Circle
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

shape.draw();
/// @resolution.name source=shape target=shape
/// @resolution.member source=shape.draw receiver=Rectangle | Circle kind=union targets=[Rectangle.draw, Circle.draw]
/// @resolution.call source=shape.draw() parameters=() return=void kind=union targets=[Rectangle.draw, Circle.draw]

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
=== annotated ===
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

=== checked ===
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.method symbol=Rectangle.draw source="draw(): void {}" slot=draw type=(this: Rectangle) => void

    draw(): void {}
    /// @type.symbol symbol=Rectangle.draw source="draw(): void {}" type=(this: Rectangle) => void

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.method symbol=Circle.draw source="draw(): void {}" slot=draw type=(this: Circle) => void

    draw(): void {}
    /// @type.symbol symbol=Circle.draw source="draw(): void {}" type=(this: Circle) => void

}

type Shape = Rectangle | Circle;
/// @type.symbol symbol=Shape source="type Shape = Rectangle | Circle" type=Rectangle | Circle
/// @definition.type symbol=Shape source="type Shape = Rectangle | Circle" value=Rectangle | Circle
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle

function draw(shape: Shape): void {
/// @type.symbol symbol=draw type=(Rectangle | Circle) => void
/// @type.symbol symbol=shape source="shape: Shape" type=Rectangle | Circle
/// @resolution.name source=Shape target=Shape

    shape.draw();
    /// @resolution.name source=shape target=shape
    /// @resolution.member source=shape.draw receiver=Rectangle | Circle kind=union targets=[Rectangle.draw, Circle.draw]
    /// @resolution.call source=shape.draw() parameters=() return=void kind=union targets=[Rectangle.draw, Circle.draw]

}
"#);
}
