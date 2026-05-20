use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_preserves_borrow_lifetime_through_type_narrowing() {
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
        DirRows::checked(),
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
/// @generic.slot symbol=read.L index=0 kind=static constraint=Lifetime
/// @type.symbol symbol=read type=<L: Lifetime>(Borrowed<Rectangle | Circle, read.L, mutable>) => int32

    if (shape is Borrowed<Rectangle, L>) {
    /// @resolution.name source=shape target=shape
    /// @type.node source=shape type=Borrowed<Rectangle, read.L, mutable>

        return shape.width;
        /// @resolution.name source=shape target=shape
        /// @resolution.member source=shape.width receiver=Borrowed<Rectangle, read.L, mutable> kind=direct target=Rectangle.width
        /// @type.node source=shape.width type=int32
    }

    return shape.radius;
    /// @resolution.name source=shape target=shape
    /// @resolution.member source=shape.radius receiver=Borrowed<Circle, read.L, mutable> kind=direct target=Circle.radius
    /// @type.node source=shape.radius type=int32
}
"#);
}

#[test]
fn test_check_joins_borrow_lifetimes_from_narrowed_branches() {
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
): Borrowed<string | int32, join(L, R)> {
    return flag ? &left.value : &right.value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
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
/// @generic.slot symbol=value.L index=0 kind=static constraint=Lifetime
/// @generic.slot symbol=value.R index=1 kind=static constraint=Lifetime
/// @type.symbol symbol=value type=<L: Lifetime, R: Lifetime>(Borrowed<Text, value.L, mutable>, Borrowed<Number, value.R, mutable>, boolean) => Borrowed<string | int32, join(value.L | value.R), mutable>

    left: Borrowed<Text, L>,
    right: Borrowed<Number, R>,
    flag: boolean,
): Borrowed<string | int32, join(L, R)> {
    return flag ? &left.value : &right.value;
    /// @resolution.name source=flag target=flag
    /// @resolution.name source=left target=left
    /// @resolution.member source=left.value receiver=Borrowed<Text, value.L, mutable> kind=direct target=Text.value
    /// @resolution.name source=right target=right
    /// @resolution.member source=right.value receiver=Borrowed<Number, value.R, mutable> kind=direct target=Number.value
    /// @type.node source="flag ? &left.value : &right.value" type=Borrowed<string | int32, join(value.L | value.R), mutable>
}
"#);
}
