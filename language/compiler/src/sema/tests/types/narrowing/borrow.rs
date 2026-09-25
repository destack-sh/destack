use crate::tests::{DirRows, TestSession};

/// Narrowing a borrowed union keeps the lifetime of the borrow.
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

function read<'a>(shape: Borrowed<Rectangle | Circle, 'a>): int32 {
    if (shape is Borrowed<Rectangle, 'a>) {
        return shape.width;
    }

    return shape.radius;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Rectangle {
    width: int32;
}

struct Circle {
    radius: int32;
}

function read<'a>(shape: &'a (Rectangle | Circle)): int32 {
    if (shape is Borrowed<Rectangle, 'a>) {
        return shape.width;
    }

    return shape.radius;
}

=== dir ===
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

function read<'a>(shape: Borrowed<Rectangle | Circle, 'a>): int32 {
/// @generic.template symbol=read parameters=('a)
/// @type.symbol symbol=read type=<'a>(&'a (Rectangle | Circle)) => int32
/// @type.symbol symbol=read.'a source='a type='a
/// @type.symbol symbol=read.shape source="shape: Borrowed<Rectangle | Circle, 'a>" type=&'a (Rectangle | Circle)
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source='a target=read.'a

    if (shape is Borrowed<Rectangle, 'a>) {
    /// @type.node source="shape is Borrowed<Rectangle, 'a>" type=boolean
    /// @type.node source=shape type=&'a (Rectangle | Circle)
    /// @resolution.name source=shape target=read.shape
    /// @resolution.guard source="shape is Borrowed<Rectangle, 'a>" kind=is value=&'a (Rectangle | Circle) target=&'a Rectangle predicate="&'a (Rectangle | Circle) is type(&'a Rectangle)" narrowed=Narrow<&'a (Rectangle | Circle), &'a Rectangle>
    /// @resolution.place source=shape placement='a lifetime='a access="mutable"
    /// @resolution.access source=shape root=read.shape
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source='a target=read.'a

        return shape.width;
        /// @type.node source=shape type=&'a Rectangle
        /// @type.node source=shape.width type=int32
        /// @resolution.name source=shape target=read.shape
        /// @resolution.member source=shape.width receiver=Narrow<&'a (Rectangle | Circle), &'a Rectangle> type=int32 kind=field target_receiver=Narrow<&'a (Rectangle | Circle), &'a Rectangle> key=width target=Rectangle.width target_type=int32
        /// @resolution.place source=shape placement='a lifetime='a access="mutable"
        /// @resolution.access source=shape root=read.shape
        /// @resolution.narrowing source=shape union=&'a (Rectangle | Circle) arms=&'a Rectangle
        /// @resolution.place source=shape.width placement='a lifetime='a access="mutable"
        /// @resolution.access source=shape.width root=read.shape keys=[width]

    }

    return shape.radius;
    /// @type.node source=shape type=&'a Circle
    /// @type.node source=shape.radius type=int32
    /// @resolution.name source=shape target=read.shape
    /// @resolution.member source=shape.radius receiver=&'a Circle type=int32 kind=field target_receiver=&'a Circle key=radius target=Circle.radius target_type=int32
    /// @resolution.place source=shape placement='a lifetime='a access="mutable"
    /// @resolution.access source=shape root=read.shape
    /// @resolution.narrowing source=shape union=&'a (Rectangle | Circle) arms=&'a Circle
    /// @resolution.place source=shape.radius placement='a lifetime='a access="mutable"
    /// @resolution.access source=shape.radius root=read.shape keys=[radius]

}
"#);
}

/// Narrowing a borrowed union keeps the access mode of the borrow.
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

function read<'a, const A: Access>(
    shape: Borrowed<Rectangle | Circle, 'a, A>,
): int32 {
    if (shape is Borrowed<Rectangle, 'a, A>) {
        return shape.width;
    }

    return shape.radius;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Rectangle {
    width: int32;
}

struct Circle {
    radius: int32;
}

function read<'a, const A: Access>(shape: Borrowed<Rectangle | Circle, 'a, A>): int32 {
    if (shape is Borrowed<Rectangle, 'a, A>) {
        return shape.width;
    }

    return shape.radius;
}

=== dir ===
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

function read<'a, const A: Access>(
/// @generic.template symbol=read parameters=('a, const A: Access)
/// @type.symbol symbol=read type=<'a, const A: Access>(WithAccess<&'a (Rectangle | Circle), A>) => int32
/// @type.symbol symbol=read.'a source='a type='a
/// @type.symbol symbol=read.A source="const A: Access" type=A
/// @resolution.name source=Access target=Access

    shape: Borrowed<Rectangle | Circle, 'a, A>,
    /// @type.symbol symbol=read.shape source="shape: Borrowed<Rectangle | Circle, 'a, A>" type=WithAccess<&'a (Rectangle | Circle), A>
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source=Circle target=Circle
    /// @resolution.name source='a target=read.'a
    /// @resolution.name source=A target=read.A

): int32 {
    if (shape is Borrowed<Rectangle, 'a, A>) {
    /// @type.node source="shape is Borrowed<Rectangle, 'a, A>" type=boolean
    /// @type.node source=shape type=WithAccess<&'a (Rectangle | Circle), A>
    /// @resolution.name source=shape target=read.shape
    /// @resolution.guard source="shape is Borrowed<Rectangle, 'a, A>" kind=is value=WithAccess<&'a (Rectangle | Circle), A> target=WithAccess<&'a Rectangle, A> predicate="WithAccess<&'a (Rectangle | Circle), A> is type(WithAccess<&'a Rectangle, A>)" narrowed=Narrow<WithAccess<&'a (Rectangle | Circle), A>, WithAccess<&'a Rectangle, A>>
    /// @resolution.place source=shape placement='a lifetime='a access=A
    /// @resolution.access source=shape root=read.shape
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source='a target=read.'a
    /// @resolution.name source=A target=read.A

        return shape.width;
        /// @type.node source=shape type=WithAccess<&'a Rectangle, A>
        /// @type.node source=shape.width type=int32
        /// @resolution.name source=shape target=read.shape
        /// @resolution.member source=shape.width receiver=Narrow<WithAccess<&'a (Rectangle | Circle), A>, WithAccess<&'a Rectangle, A>> type=int32 kind=field target_receiver=Narrow<WithAccess<&'a (Rectangle | Circle), A>, WithAccess<&'a Rectangle, A>> key=width target=Rectangle.width target_type=int32
        /// @resolution.place source=shape placement='a lifetime='a access=A
        /// @resolution.access source=shape root=read.shape
        /// @resolution.place source=shape.width placement='a lifetime='a access=A
        /// @resolution.access source=shape.width root=read.shape keys=[width]

    }

    return shape.radius;
    /// @type.node source=shape type=WithAccess<&'a Circle, A>
    /// @type.node source=shape.radius type=int32
    /// @resolution.name source=shape target=read.shape
    /// @resolution.member source=shape.radius receiver=WithAccess<&'a Circle, A> type=int32 kind=field target_receiver=WithAccess<&'a Circle, A> key=radius target=Circle.radius target_type=int32
    /// @resolution.place source=shape placement='a lifetime='a access=A
    /// @resolution.access source=shape root=read.shape
    /// @resolution.narrowing source=shape union=WithAccess<&'a (Rectangle | Circle), A> arms=WithAccess<&'a Circle, A>
    /// @resolution.place source=shape.radius placement='a lifetime='a access=A
    /// @resolution.access source=shape.radius root=read.shape keys=[radius]

}
"#);
}

/// Two conditional branches join the lifetimes of the borrows they return.
#[test]
fn test_narrowed_branches_join_borrow_lifetimes() {
    let session = TestSession::single(
        r#"
struct Text {
    value: string;
}

function value<'a, 'b>(
    left: Borrowed<Text, 'a>,
    right: Borrowed<Text, 'b>,
    flag: boolean,
): Borrowed<string, 'a | 'b> {
    return flag ? (&left.value) : (&right.value);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Text {
    value: string;
}

function value<'a, 'b>(
    left: &'a Text,
    right: &'b Text,
    flag: boolean,
): Borrowed<string, 'a | 'b, "mutable"> {
    return flag ? &left.value : &right.value;
}

=== dir ===
struct Text {
/// @type.symbol symbol=Text type=Text
/// @definition.struct symbol=Text
/// @definition.field symbol=Text.value source="value: string" key=value type=string

    value: string;
    /// @type.symbol symbol=Text.value source="value: string" type=string

}

function value<'a, 'b>(
/// @generic.template symbol=value parameters=('a, 'b)
/// @type.symbol symbol=value type=<'a, 'b>(&'a Text, &'b Text, boolean) => &string
/// @type.symbol symbol=value.'a source='a type='a
/// @type.symbol symbol=value.'b source='b type='b

    left: Borrowed<Text, 'a>,
    /// @type.symbol symbol=value.left source="left: Borrowed<Text, 'a>" type=&'a Text
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Text target=Text
    /// @resolution.name source='a target=value.'a

    right: Borrowed<Text, 'b>,
    /// @type.symbol symbol=value.right source="right: Borrowed<Text, 'b>" type=&'b Text
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Text target=Text
    /// @resolution.name source='b target=value.'b

    flag: boolean,
    /// @type.symbol symbol=value.flag source="flag: boolean" type=boolean

): Borrowed<string, 'a | 'b> {
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source='a target=value.'a
/// @resolution.name source='b target=value.'b

    return flag ? (&left.value) : (&right.value);
    /// @type.node source="flag ? (&left.value) : (&right.value)" type=&string
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=value.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=value.flag
    /// @type.node source=&left.value type=&'a string
    /// @type.node source=left type=&'a Text
    /// @type.node source=left.value type=string
    /// @resolution.name source=left target=value.left
    /// @resolution.member source=left.value receiver=&'a Text type=string kind=field target_receiver=&'a Text key=value target=Text.value target_type=string
    /// @resolution.place source=left placement='a lifetime='a access="mutable"
    /// @resolution.access source=left root=value.left
    /// @resolution.place source=left.value placement='a lifetime='a access="mutable"
    /// @resolution.access source=left.value root=value.left keys=[value]
    /// @type.node source=&right.value type=&'b string
    /// @type.node source=right type=&'b Text
    /// @type.node source=right.value type=string
    /// @resolution.name source=right target=value.right
    /// @resolution.member source=right.value receiver=&'b Text type=string kind=field target_receiver=&'b Text key=value target=Text.value target_type=string
    /// @resolution.place source=right placement='b lifetime='b access="mutable"
    /// @resolution.access source=right root=value.right
    /// @resolution.place source=right.value placement='b lifetime='b access="mutable"
    /// @resolution.access source=right.value root=value.right keys=[value]

}
"#,
        r#"

"#);
}
