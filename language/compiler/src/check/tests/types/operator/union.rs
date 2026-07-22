use crate::tests::{DirRows, TestSession};

#[test]
fn test_union_accepts_each_member() {
    let session = TestSession::single(
        r#"
let value: string | int32 = "hello";
value = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
let value: string | int32 = "hello" as string | int32;
value = 42 as string | int32;

=== checked ===
let value: string | int32 = "hello";
/// @type.symbol symbol=value source=value type=string | int32
/// @resolution.pattern source=value kind=binding target=value

value = 42;
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=string | int32
"#,
    );
}

#[test]
fn test_union_accepts_declared_member() {
    let session = TestSession::single(
        r#"
const text: string = "hello";
const value: string | int32 = text;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const text: string = "hello";
const value: string | int32 = text as string | int32;

=== checked ===
const text: string = "hello";
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text

const value: string | int32 = text;
/// @type.symbol symbol=value source=value type=string | int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=text target=text
"#,
    );
}

#[test]
fn test_union_flattens_nested_aliases() {
    let session = TestSession::single(
        r#"
type A = { a: int32 } | { b: string };
type B = A | { c: boolean };

const value: B = { c: true };
value satisfies { a: int32 } | { b: string } | { c: boolean };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type A = { a: int32 } | { b: string };
type B = A | { c: boolean };

const value: B = { c: true } as B;
value satisfies { a: int32 } | { b: string } | { c: boolean };

=== checked ===
type A = { a: int32 } | { b: string };
/// @type.symbol symbol=A source="type A = { a: int32 } | { b: string }" type={ a: int32 } | { b: string }
/// @definition.type symbol=A source="type A = { a: int32 } | { b: string }" value={ a: int32 } | { b: string }

type B = A | { c: boolean };
/// @type.symbol symbol=B source="type B = A | { c: boolean }" type=A | { c: boolean }
/// @definition.type symbol=B source="type B = A | { c: boolean }" value=A | { c: boolean }
/// @resolution.name source=A target=A

const value: B = { c: true };
/// @type.symbol symbol=value source=value type=B reduced=A | { c: boolean }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=B target=B

value satisfies { a: int32 } | { b: string } | { c: boolean };
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_union_removes_never() {
    let session = TestSession::single(
        r#"
type A = never | string;

const value: A = "hello";
value satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type A = never | string;

const value: A = "hello";
value satisfies string;

=== checked ===
type A = never | string;
/// @type.symbol symbol=A source="type A = never | string" type=string
/// @definition.type symbol=A source="type A = never | string" value=string

const value: A = "hello";
/// @type.symbol symbol=value source=value type=A reduced=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=A target=A

value satisfies string;
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_void_union_with_never_keeps_void() {
    let session = TestSession::single(
        r#"
type Value = void | never;

const value: Value = ();
value satisfies void;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = void | never;

const value: Value = ();
value satisfies void;

=== checked ===
type Value = void | never;
/// @type.symbol symbol=Value source="type Value = void | never" type=void
/// @definition.type symbol=Value source="type Value = void | never" value=void

const value: Value = ();
/// @type.symbol symbol=value source=value type=Value reduced=void
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

value satisfies void;
/// @resolution.name source=value target=value
"#,
    );
}

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

let shape: Rectangle | Circle = Rectangle {} as Rectangle | Circle;
shape.draw();

=== checked ===
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.method symbol=Rectangle.draw source="draw(): void {}" slot=draw type=<Rectangle.draw.'l0>(this: &Rectangle.draw.'l0 exclusive this) => void

    draw(): void {}
    /// @generic.template symbol=Rectangle.draw parameters=('l0)
    /// @type.symbol symbol=Rectangle.draw source="draw(): void {}" type=<Rectangle.draw.'l0>(this: &Rectangle.draw.'l0 exclusive this) => void

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.method symbol=Circle.draw source="draw(): void {}" slot=draw type=<Circle.draw.'l0>(this: &Circle.draw.'l0 exclusive this) => void

    draw(): void {}
    /// @generic.template symbol=Circle.draw parameters=('l0)
    /// @type.symbol symbol=Circle.draw source="draw(): void {}" type=<Circle.draw.'l0>(this: &Circle.draw.'l0 exclusive this) => void

}

let shape: Rectangle | Circle = Rectangle {};
/// @type.symbol symbol=shape source=shape type=Rectangle | Circle
/// @resolution.pattern source=shape kind=binding target=shape
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

shape.draw();
/// @resolution.name source=shape target=shape
/// @resolution.member source=shape.draw receiver=Rectangle | Circle kind=universal targets=[Rectangle.draw, Circle.draw]
/// @resolution.call source=shape.draw() parameters=() return=void kind=universal targets=[Rectangle.draw, Circle.draw]
"#);
}

#[test]
fn test_union_alias_parameter_dispatches_each_variant() {
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
/// @definition.method symbol=Rectangle.draw source="draw(): void {}" slot=draw type=<Rectangle.draw.'l0>(this: &Rectangle.draw.'l0 exclusive this) => void

    draw(): void {}
    /// @generic.template symbol=Rectangle.draw parameters=('l0)
    /// @type.symbol symbol=Rectangle.draw source="draw(): void {}" type=<Rectangle.draw.'l0>(this: &Rectangle.draw.'l0 exclusive this) => void

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.method symbol=Circle.draw source="draw(): void {}" slot=draw type=<Circle.draw.'l0>(this: &Circle.draw.'l0 exclusive this) => void

    draw(): void {}
    /// @generic.template symbol=Circle.draw parameters=('l0)
    /// @type.symbol symbol=Circle.draw source="draw(): void {}" type=<Circle.draw.'l0>(this: &Circle.draw.'l0 exclusive this) => void

}

type Shape = Rectangle | Circle;
/// @type.symbol symbol=Shape source="type Shape = Rectangle | Circle" type=Rectangle | Circle
/// @definition.type symbol=Shape source="type Shape = Rectangle | Circle" value=Rectangle | Circle
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle

function draw(shape: Shape): void {
/// @type.symbol symbol=draw type=(Shape) => void
/// @type.symbol symbol=draw.shape source="shape: Shape" type=Shape reduced=Rectangle | Circle
/// @resolution.name source=Shape target=Shape

    shape.draw();
    /// @resolution.name source=shape target=draw.shape
    /// @resolution.member source=shape.draw receiver=Rectangle | Circle kind=universal targets=[Rectangle.draw, Circle.draw]
    /// @resolution.call source=shape.draw() parameters=() return=void kind=universal targets=[Rectangle.draw, Circle.draw]

}
"#);
}
