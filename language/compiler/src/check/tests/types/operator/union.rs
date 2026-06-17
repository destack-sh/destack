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

value = 42;
/// @resolution.name source=value target=value
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

const value: string | int32 = text;
/// @type.symbol symbol=value source=value type=string | int32
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
/// @type.symbol symbol=B source="type B = A | { c: boolean }" type={ a: int32 } | { b: string } | { c: boolean }
/// @definition.type symbol=B source="type B = A | { c: boolean }" value={ a: int32 } | { b: string } | { c: boolean }
/// @resolution.name source=A target=A

const value: B = { c: true };
/// @type.symbol symbol=value source=value type={ a: int32 } | { b: string } | { c: boolean }
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
/// @type.symbol symbol=value source=value type=string
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
/// @type.symbol symbol=value source=value type=void
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

function draw<T0: Shape>(shape: T0): void {
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
/// @generic.template symbol=draw parameters=[T0: Shape]
/// @type.symbol symbol=draw type=<draw.T0: Shape>(draw.T0) => void
/// @type.symbol symbol=shape source="shape: Shape" type=draw.T0
/// @resolution.name source=Shape target=Shape

    shape.draw();
    /// @resolution.name source=shape target=shape
    /// @resolution.member source=shape.draw receiver=draw.T0 kind=union targets=[Rectangle.draw, Circle.draw]
    /// @resolution.call source=shape.draw() parameters=() return=void kind=union targets=[Rectangle.draw, Circle.draw] receiver=draw.T0

}
"#);
}
