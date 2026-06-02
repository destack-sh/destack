use crate::tests::{DirRows, TestSession};

#[test]
fn test_type_narrowing_preserves_borrow_lifetime() {
    let session = TestSession::single(
        r#"
struct Rectangle {
    width: int32;
}

struct Circle {
    radius: int32;
}

function read<L: Lifetime>(shape: Borrowed<Rectangle | Circle, L>): int32 {
    if (shape is Borrowed<Rectangle, L>) {
        return shape.width;
    }

    return shape.radius;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle

    width: int32;
    /// @type.symbol symbol=Rectangle.width type=int32

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle

    radius: int32;
    /// @type.symbol symbol=Circle.radius type=int32

}

function read<L: Lifetime>(shape: Borrowed<Rectangle | Circle, L>): int32 {
/// @type.symbol symbol=read type=(Borrowed<Rectangle | Circle, read.L, "mutable">) => int32
/// @generic.template symbol=read parameters=[comptime L: memory.lifetime.Lifetime]
/// @type.symbol symbol=shape type=Borrowed<Rectangle | Circle, read.L, "mutable">

    if (shape is Borrowed<Rectangle, L>) {
    /// @type.node type=void
    /// @type.node source="shape is Borrowed<Rectangle, L>" type=boolean
    /// @type.node source=shape type=Borrowed<Rectangle | Circle, read.L, "mutable">
    /// @resolution.name source=shape target=shape

        return shape.width;
        /// @type.node source=shape type=Borrowed<Rectangle, read.L, "mutable">
        /// @type.node source=shape.width type=int32
        /// @resolution.name source=shape target=shape
        /// @resolution.member source=shape.width receiver=Borrowed<Rectangle, read.L, "mutable"> kind=symbol target=Rectangle.width

    }

    return shape.radius;
    /// @type.node source=shape type=Borrowed<Circle, read.L, "mutable">
    /// @type.node source=shape.radius type=int32
    /// @resolution.name source=shape target=shape
    /// @resolution.member source=shape.radius receiver=Borrowed<Circle, read.L, "mutable"> kind=symbol target=Circle.radius

}
"#);
}

#[test]
fn test_narrowed_branches_join_borrow_lifetimes() {
    let session = TestSession::single(
        r#"
struct Text {
    value: string;
}

struct Number {
    value: int32;
}

function value<L: Lifetime, R: Lifetime>(
    left: Borrowed<Text, L>,
    right: Borrowed<Number, R>,
    flag: boolean,
): Borrowed<string | int32, L | R> {
    return flag ? (&left.value) : (&right.value);
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
struct Text {
/// @type.symbol symbol=Text type=Text

    value: string;
    /// @type.symbol symbol=Text.value type=string

}

struct Number {
/// @type.symbol symbol=Number type=Number

    value: int32;
    /// @type.symbol symbol=Number.value type=int32

}

function value<L: Lifetime, R: Lifetime>(
/// @type.symbol symbol=value type=(Borrowed<Text, value.L, "mutable">, Borrowed<Number, value.R, "mutable">, boolean) => Borrowed<string | int32, value.L | value.R, "mutable">
/// @generic.template symbol=value parameters=[comptime L: memory.lifetime.Lifetime, comptime R: memory.lifetime.Lifetime]

    left: Borrowed<Text, L>,
    /// @type.symbol symbol=left type=Borrowed<Text, value.L, "mutable">

    right: Borrowed<Number, R>,
    /// @type.symbol symbol=right type=Borrowed<Number, value.R, "mutable">

    flag: boolean,
    /// @type.symbol symbol=flag type=boolean

): Borrowed<string | int32, L | R> {
    return flag ? (&left.value) : (&right.value);
    /// @type.node source="flag ? (&left.value) : (&right.value)" type=Borrowed<string | int32, value.L | value.R, "mutable">
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=flag
    /// @type.node source=(&left.value) type=Borrowed<string, value.L, "mutable">
    /// @type.node source=&left.value type=Borrowed<string, value.L, "mutable">
    /// @type.node source=left type=Borrowed<Text, value.L, "mutable">
    /// @type.node source=left.value type=string
    /// @resolution.name source=left target=left
    /// @resolution.member source=left.value receiver=Borrowed<Text, value.L, "mutable"> kind=symbol target=Text.value
    /// @type.node source=(&right.value) type=Borrowed<int32, value.R, "mutable">
    /// @type.node source=&right.value type=Borrowed<int32, value.R, "mutable">
    /// @type.node source=right type=Borrowed<Number, value.R, "mutable">
    /// @type.node source=right.value type=int32
    /// @resolution.name source=right target=right
    /// @resolution.member source=right.value receiver=Borrowed<Number, value.R, "mutable"> kind=symbol target=Number.value

}
"#);
}
