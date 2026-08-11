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

function read<'a>(shape: Borrowed<Rectangle | Circle, 'a>): int32 {
    if (shape is Borrowed<Rectangle, 'a>) {
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

function read<'a>(shape: Borrowed<Rectangle | Circle, 'a>): int32 {
    if (shape is Borrowed<Rectangle, 'a>) {
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

function read<'a>(shape: Borrowed<Rectangle | Circle, 'a>): int32 {
/// @generic.template symbol=read parameters=('a)
/// @type.symbol symbol=read type=<'a>(Borrowed<Rectangle | Circle, 'a>) => int32
/// @type.symbol symbol=read.'a source='a type='a
/// @type.symbol symbol=read.shape source="shape: Borrowed<Rectangle | Circle, 'a>" type=Borrowed<Rectangle | Circle, 'a>
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source='a target=read.'a

    if (shape is Borrowed<Rectangle, 'a>) {
    /// @type.node source="shape is Borrowed<Rectangle, 'a>" type=boolean
    /// @type.node source=shape type=Borrowed<Rectangle | Circle, 'a>
    /// @resolution.name source=shape target=read.shape
    /// @resolution.guard source="shape is Borrowed<Rectangle, 'a>" kind=is value=&'a Rectangle | Circle target=Borrowed<Rectangle, 'a, "mutable"> predicate="&'a Rectangle | Circle is type(Borrowed<Rectangle, 'a, \"mutable\">)" narrowed=Narrow<&'a Rectangle | Circle, Borrowed<Rectangle, 'a, "mutable">>
    /// @resolution.place source=shape placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=shape root=read.shape
    /// @generic.instance source=shape id="Borrowed<Rectangle | Circle, 'a>"
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source='a target=read.'a

        return shape.width;
        /// @type.node source=shape type=Narrow<&'a Rectangle | Circle, Borrowed<Rectangle, 'a, "mutable">>
        /// @type.node source=shape.width type=int32
        /// @resolution.name source=shape target=read.shape
        /// @resolution.member source=shape.width receiver=Narrow<&'a Rectangle | Circle, Borrowed<Rectangle, 'a, "mutable">> type=int32 kind=field target_receiver=Narrow<&'a Rectangle | Circle, Borrowed<Rectangle, 'a, "mutable">> key=width target=Rectangle.width target_type=int32
        /// @resolution.place source=shape placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=shape root=read.shape
        /// @resolution.place source=shape.width placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=shape.width root=read.shape keys=[width]
        /// @generic.instance source=shape id="Borrowed<Rectangle, 'a, \"mutable\">"

    }

    return shape.radius;
    /// @type.node source=shape type=&'a Circle
    /// @type.node source=shape.radius type=int32
    /// @resolution.name source=shape target=read.shape
    /// @resolution.member source=shape.radius receiver=&'a Circle type=int32 kind=field target_receiver=&'a Circle key=radius target=Circle.radius target_type=int32
    /// @resolution.place source=shape placement="local" lifetime='a access="mutable"
    /// @resolution.access source=shape root=read.shape
    /// @resolution.place source=shape.radius placement="local" lifetime='a access="mutable"
    /// @resolution.access source=shape.radius root=read.shape keys=[radius]

}

/// @generic.instance id="Borrowed<Rectangle | Circle, 'a>" template=memory.borrow.Borrowed arguments=(Rectangle | Circle, 'a)
/// @generic.instance id="Borrowed<Rectangle, 'a, \"mutable\">" template=memory.borrow.Borrowed arguments=(Rectangle, 'a, "mutable")
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

function read<'a, comptime A: Access>(
    shape: Borrowed<Rectangle | Circle, 'a, A>,
): int32 {
    if (shape is Borrowed<Rectangle, 'a, A>) {
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

function read<'a, comptime A: Access>(shape: Borrowed<Rectangle | Circle, 'a, A>): int32 {
    if (shape is Borrowed<Rectangle, 'a, A>) {
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

function read<'a, comptime A: Access>(
/// @generic.template symbol=read parameters=('a, comptime A: Access)
/// @type.symbol symbol=read type=<'a, comptime A: Access>(Borrowed<Rectangle | Circle, 'a, A>) => int32
/// @type.symbol symbol=read.'a source='a type='a
/// @type.symbol symbol=read.A source="comptime A: Access" type=A
/// @resolution.name source=Access target=memory.access.Access

    shape: Borrowed<Rectangle | Circle, 'a, A>,
    /// @type.symbol symbol=read.shape source="shape: Borrowed<Rectangle | Circle, 'a, A>" type=Borrowed<Rectangle | Circle, 'a, A>
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source=Circle target=Circle
    /// @resolution.name source='a target=read.'a
    /// @resolution.name source=A target=read.A

): int32 {
    if (shape is Borrowed<Rectangle, 'a, A>) {
    /// @type.node source="shape is Borrowed<Rectangle, 'a, A>" type=boolean
    /// @type.node source=shape type=Borrowed<Rectangle | Circle, 'a, A>
    /// @resolution.name source=shape target=read.shape
    /// @resolution.guard source="shape is Borrowed<Rectangle, 'a, A>" kind=is value=Borrowed<Rectangle | Circle, 'a, A> target=Borrowed<Rectangle, 'a, A> predicate="Borrowed<Rectangle | Circle, 'a, A> is type(Borrowed<Rectangle, 'a, A>)" narrowed=Narrow<Borrowed<Rectangle | Circle, 'a, A>, Borrowed<Rectangle, 'a, A>>
    /// @resolution.place source=shape placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=shape root=read.shape
    /// @generic.instance source=shape id="Borrowed<Rectangle | Circle, 'a, A>"
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Rectangle target=Rectangle
    /// @resolution.name source='a target=read.'a
    /// @resolution.name source=A target=read.A

        return shape.width;
        /// @type.node source=shape type=Narrow<Borrowed<Rectangle | Circle, 'a, A>, Borrowed<Rectangle, 'a, A>>
        /// @type.node source=shape.width type=int32
        /// @resolution.name source=shape target=read.shape
        /// @resolution.member source=shape.width receiver=Narrow<Borrowed<Rectangle | Circle, 'a, A>, Borrowed<Rectangle, 'a, A>> type=int32 kind=field target_receiver=Narrow<Borrowed<Rectangle | Circle, 'a, A>, Borrowed<Rectangle, 'a, A>> key=width target=Rectangle.width target_type=int32
        /// @resolution.place source=shape placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=shape root=read.shape
        /// @resolution.place source=shape.width placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=shape.width root=read.shape keys=[width]
        /// @generic.instance source=shape id="Borrowed<Rectangle, 'a, A>"

    }

    return shape.radius;
    /// @type.node source=shape type=Borrowed<Circle, 'a, A>
    /// @type.node source=shape.radius type=int32
    /// @resolution.name source=shape target=read.shape
    /// @resolution.member source=shape.radius receiver=Borrowed<Circle, 'a, A> type=int32 kind=field target_receiver=Borrowed<Circle, 'a, A> key=radius target=Circle.radius target_type=int32
    /// @resolution.place source=shape placement="local" lifetime='a access=A
    /// @resolution.access source=shape root=read.shape
    /// @resolution.place source=shape.radius placement="local" lifetime='a access=A
    /// @resolution.access source=shape.radius root=read.shape keys=[radius]

}

/// @generic.instance id="Borrowed<Rectangle | Circle, 'a, A>" template=memory.borrow.Borrowed arguments=(Rectangle | Circle, 'a, A)
/// @generic.instance id="Borrowed<Rectangle, 'a, A>" template=memory.borrow.Borrowed arguments=(Rectangle, 'a, A)
"#);
}

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Text {
    value: string;
}

function value<'a, 'b>(
    left: Borrowed<Text, 'a>,
    right: Borrowed<Text, 'b>,
    flag: boolean,
): Borrowed<string, 'a | 'b> {
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

function value<'a, 'b>(
/// @generic.template symbol=value parameters=('a, 'b)
/// @type.symbol symbol=value type=<'a, 'b>(Borrowed<Text, 'a>, Borrowed<Text, 'b>, boolean) => Borrowed<string, 'a | 'b>
/// @type.symbol symbol=value.'a source='a type='a
/// @type.symbol symbol=value.'b source='b type='b

    left: Borrowed<Text, 'a>,
    /// @type.symbol symbol=value.left source="left: Borrowed<Text, 'a>" type=Borrowed<Text, 'a>
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Text target=Text
    /// @resolution.name source='a target=value.'a

    right: Borrowed<Text, 'b>,
    /// @type.symbol symbol=value.right source="right: Borrowed<Text, 'b>" type=Borrowed<Text, 'b>
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Text target=Text
    /// @resolution.name source='b target=value.'b

    flag: boolean,
    /// @type.symbol symbol=value.flag source="flag: boolean" type=boolean

): Borrowed<string, 'a | 'b> {
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source='a target=value.'a
/// @resolution.name source='b target=value.'b

    return flag ? (&left.value) : (&right.value);
    /// @type.node source="flag ? (&left.value) : (&right.value)" type=Borrowed<string, 'a | 'b>
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=value.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=value.flag
    /// @generic.instance source="flag ? (&left.value) : (&right.value)" id="Borrowed<string, 'a | 'b>"
    /// @type.node source=&left.value type=&'frame string
    /// @type.node source=left type=Borrowed<Text, 'a>
    /// @type.node source=left.value type=string
    /// @resolution.name source=left target=value.left
    /// @resolution.member source=left.value receiver=Borrowed<Text, 'a> type=string kind=field target_receiver=Borrowed<Text, 'a> key=value target=Text.value target_type=string
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=value.left
    /// @resolution.place source=left.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left.value root=value.left keys=[value]
    /// @generic.instance source=left id="Borrowed<Text, 'a>"
    /// @type.node source=&right.value type=&'frame string
    /// @type.node source=right type=Borrowed<Text, 'b>
    /// @type.node source=right.value type=string
    /// @resolution.name source=right target=value.right
    /// @resolution.member source=right.value receiver=Borrowed<Text, 'b> type=string kind=field target_receiver=Borrowed<Text, 'b> key=value target=Text.value target_type=string
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=value.right
    /// @resolution.place source=right.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right.value root=value.right keys=[value]
    /// @generic.instance source=right id="Borrowed<Text, 'b>"

}

/// @generic.instance id="Borrowed<Text, 'a>" template=memory.borrow.Borrowed arguments=(Text, 'a)
/// @generic.instance id="Borrowed<Text, 'b>" template=memory.borrow.Borrowed arguments=(Text, 'b)
/// @generic.instance id="Borrowed<string, 'a | 'b>" template=memory.borrow.Borrowed arguments=(string, 'a | 'b)
"#);
}
