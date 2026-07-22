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

function read<comptime L: Lifetime>(shape: Borrowed<Rectangle | Circle, L>): int32 {
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
=== annotated ===
struct Rectangle {
    width: int32;
}

struct Circle {
    radius: int32;
}

function read<comptime L: Lifetime>(shape: Borrowed<Rectangle | Circle, L, "mutable">): int32 {
    if (shape is Borrowed<Rectangle, L>) {
        return shape.width;
    }

    return shape.radius;
}

=== checked ===
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.width source="width: int32" key=width type=int32

    width: int32;
    /// @type.symbol symbol=Rectangle.width source="width: int32" type=int32

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.field symbol=Circle.radius source="radius: int32" key=radius type=int32

    radius: int32;
    /// @type.symbol symbol=Circle.radius source="radius: int32" type=int32

}

function read<comptime L: Lifetime>(shape: Borrowed<Rectangle | Circle, L>): int32 {
/// @generic.template symbol=read parameters=(comptime L: Lifetime)
/// @type.symbol symbol=read type=<comptime L: Lifetime>(Borrowed<Rectangle | Circle, L, "mutable">) => int32
/// @type.symbol symbol=read.L source="comptime L: Lifetime" type=L
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime
/// @type.symbol symbol=read.shape source="shape: Borrowed<Rectangle | Circle, L>" type=Borrowed<Rectangle | Circle, L, "mutable">
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=L target=read.L

    if (shape is Borrowed<Rectangle, L>) {
    /// @type.node source="shape is Borrowed<Rectangle, L>" type=boolean
    /// @type.node source=shape type=Borrowed<Rectangle | Circle, L, "mutable">
    /// @resolution.name source=shape target=read.shape
    /// @resolution.guard source="shape is Borrowed<Rectangle, L>" kind=is value=Borrowed<Rectangle | Circle, L, "mutable"> target=Borrowed<Rectangle, L, "mutable"> predicate="Borrowed<Rectangle | Circle, L, \"mutable\"> is type(Borrowed<Rectangle, L, \"mutable\">)" narrowed=Borrowed<Rectangle, L, "mutable">
    /// @generic.instance source=shape id="Borrowed<Rectangle | Circle, L, \"mutable\">"
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source=L target=read.L

        return shape.width;
        /// @type.node source=shape type=Borrowed<Rectangle, L, "mutable">
        /// @type.node source=shape.width type=int32
        /// @resolution.name source=shape target=read.shape
        /// @resolution.member source=shape.width receiver=Borrowed<Rectangle, L, "mutable"> kind=symbol target=Rectangle.width

    }

    return shape.radius;
    /// @type.node source=shape type=Borrowed<Circle, L, "mutable">
    /// @type.node source=shape.radius type=int32
    /// @resolution.name source=shape target=read.shape
    /// @resolution.member source=shape.radius receiver=Borrowed<Circle, L, "mutable"> kind=symbol target=Circle.radius

}

/// @generic.instance id="Borrowed<Rectangle | Circle, L, \"mutable\">" template=memory.borrow.Borrowed arguments=(Rectangle | Circle, L, "mutable")
"#);
}

#[test]
fn test_type_narrowing_preserves_borrow_access() {
    let session = TestSession::single(
        r#"
struct Rectangle {
    width: int32;
}

struct Circle {
    radius: int32;
}

function read<comptime L: Lifetime, comptime A: Access>(
    shape: Borrowed<Rectangle | Circle, L, A>,
): int32 {
    if (shape is Borrowed<Rectangle, L, A>) {
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
=== annotated ===
struct Rectangle {
    width: int32;
}

struct Circle {
    radius: int32;
}

function read<comptime L: Lifetime, comptime A: Access>(
    shape: Borrowed<Rectangle | Circle, L, A>,
): int32 {
    if (shape is Borrowed<Rectangle, L, A>) {
        return shape.width;
    }

    return shape.radius;
}

=== checked ===
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.width source="width: int32" key=width type=int32

    width: int32;
    /// @type.symbol symbol=Rectangle.width source="width: int32" type=int32

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.field symbol=Circle.radius source="radius: int32" key=radius type=int32

    radius: int32;
    /// @type.symbol symbol=Circle.radius source="radius: int32" type=int32

}

function read<comptime L: Lifetime, comptime A: Access>(
/// @generic.template symbol=read parameters=(comptime L: Lifetime, comptime A: Access)
/// @type.symbol symbol=read type=<comptime L: Lifetime, comptime A: Access>(Borrowed<Rectangle | Circle, L, A>) => int32
/// @type.symbol symbol=read.L source="comptime L: Lifetime" type=L
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime
/// @type.symbol symbol=read.A source="comptime A: Access" type=A
/// @resolution.name source=Access target=memory.access.Access

    shape: Borrowed<Rectangle | Circle, L, A>,
    /// @type.symbol symbol=read.shape source="shape: Borrowed<Rectangle | Circle, L, A>" type=Borrowed<Rectangle | Circle, L, A>
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source=Circle target=Circle
    /// @resolution.name source=L target=read.L
    /// @resolution.name source=A target=read.A

): int32 {
    if (shape is Borrowed<Rectangle, L, A>) {
    /// @type.node source="shape is Borrowed<Rectangle, L, A>" type=boolean
    /// @type.node source=shape type=Borrowed<Rectangle | Circle, L, A>
    /// @resolution.name source=shape target=read.shape
    /// @resolution.guard source="shape is Borrowed<Rectangle, L, A>" kind=is value=Borrowed<Rectangle | Circle, L, A> target=Borrowed<Rectangle, L, A> predicate="Borrowed<Rectangle | Circle, L, A> is type(Borrowed<Rectangle, L, A>)" narrowed=Borrowed<Rectangle, L, A>
    /// @generic.instance source=shape id="Borrowed<Rectangle | Circle, L, A>"
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source=L target=read.L
    /// @resolution.name source=A target=read.A

        return shape.width;
        /// @type.node source=shape type=Borrowed<Rectangle, L, A>
        /// @type.node source=shape.width type=int32
        /// @resolution.name source=shape target=read.shape
        /// @resolution.member source=shape.width receiver=Borrowed<Rectangle, L, A> kind=symbol target=Rectangle.width

    }

    return shape.radius;
    /// @type.node source=shape type=Borrowed<Circle, L, A>
    /// @type.node source=shape.radius type=int32
    /// @resolution.name source=shape target=read.shape
    /// @resolution.member source=shape.radius receiver=Borrowed<Circle, L, A> kind=symbol target=Circle.radius

}

/// @generic.instance id="Borrowed<Rectangle | Circle, L, A>" template=memory.borrow.Borrowed arguments=(Rectangle | Circle, L, A)
"#);
}

#[test]
fn test_narrowed_branches_join_borrow_lifetimes() {
    let session = TestSession::single(
        r#"
struct Text {
    value: string;
}

function value<L: Lifetime, R: Lifetime>(
    left: Borrowed<Text, L>,
    right: Borrowed<Text, R>,
    flag: boolean,
): Borrowed<string, L | R> {
    return flag ? (&left.value) : (&right.value);
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Text {
    value: string;
}

function value<L: Lifetime, R: Lifetime>(
    left: Borrowed<Text, L, "mutable">,
    right: Borrowed<Text, R, "mutable">,
    flag: boolean,
): Borrowed<string, L | R, "mutable"> {
    return flag ? &left.value : &right.value;
}

=== checked ===
struct Text {
/// @type.symbol symbol=Text type=Text
/// @definition.struct symbol=Text
/// @definition.field symbol=Text.value source="value: string" key=value type=string

    value: string;
    /// @type.symbol symbol=Text.value source="value: string" type=string

}

function value<L: Lifetime, R: Lifetime>(
/// @generic.template symbol=value parameters=(L: Lifetime, R: Lifetime)
/// @type.symbol symbol=value type=<L: Lifetime, R: Lifetime>(Borrowed<Text, L, "mutable">, Borrowed<Text, R, "mutable">, boolean) => Borrowed<string, L | R, "mutable">
/// @type.symbol symbol=value.L source="L: Lifetime" type=L
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime
/// @type.symbol symbol=value.R source="R: Lifetime" type=R
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime

    left: Borrowed<Text, L>,
    /// @type.symbol symbol=value.left source="left: Borrowed<Text, L>" type=Borrowed<Text, L, "mutable">
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Text target=Text
    /// @resolution.name source=L target=value.L

    right: Borrowed<Text, R>,
    /// @type.symbol symbol=value.right source="right: Borrowed<Text, R>" type=Borrowed<Text, R, "mutable">
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Text target=Text
    /// @resolution.name source=R target=value.R

    flag: boolean,
    /// @type.symbol symbol=value.flag source="flag: boolean" type=boolean

): Borrowed<string, L | R> {
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=L target=value.L
/// @resolution.name source=R target=value.R

    return flag ? (&left.value) : (&right.value);
    /// @type.node source="flag ? (&left.value) : (&right.value)" type=Borrowed<string, L | R, "mutable">
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=value.flag
    /// @generic.instance source="flag ? (&left.value) : (&right.value)" id="Borrowed<string, L | R, \"mutable\">"
    /// @type.node source=&left.value type=Borrowed<string, L, "mutable">
    /// @type.node source=left type=Borrowed<Text, L, "mutable">
    /// @type.node source=left.value type=string
    /// @resolution.name source=left target=value.left
    /// @resolution.member source=left.value receiver=Borrowed<Text, L, "mutable"> kind=symbol target=Text.value
    /// @generic.instance source=left id="Borrowed<Text, L, \"mutable\">"
    /// @type.node source=&right.value type=Borrowed<string, R, "mutable">
    /// @type.node source=right type=Borrowed<Text, R, "mutable">
    /// @type.node source=right.value type=string
    /// @resolution.name source=right target=value.right
    /// @resolution.member source=right.value receiver=Borrowed<Text, R, "mutable"> kind=symbol target=Text.value
    /// @generic.instance source=right id="Borrowed<Text, R, \"mutable\">"

}

/// @generic.instance id="Borrowed<Text, L, \"mutable\">" template=memory.borrow.Borrowed arguments=(Text, L, "mutable")
/// @generic.instance id="Borrowed<Text, R, \"mutable\">" template=memory.borrow.Borrowed arguments=(Text, R, "mutable")
/// @generic.instance id="Borrowed<string, L | R, \"mutable\">" template=memory.borrow.Borrowed arguments=(string, L | R, "mutable")
"#);
}
