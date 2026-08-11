use crate::tests::{DirRows, TestSession};

#[test]
fn test_value_generic_argument_still_works_when_parameter_is_static() {
    let session = TestSession::single(
        r#"
type Slots<comptime N: usize> = [uint8; N];
type Bytes = Slots<16>;

declare const bytes: Bytes;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
type Slots<comptime N: usize> = [uint8; N];
type Bytes = Slots<16>;

declare const bytes: [uint8; 16];

=== checked ===
type Slots<comptime N: usize> = [uint8; N];
/// @generic.template symbol=Slots parameters=(comptime N: usize)
/// @type.symbol symbol=Slots source="type Slots<comptime N: usize> = [uint8; N]" type=FixedArray<uint8, N>
/// @definition.type symbol=Slots source="type Slots<comptime N: usize> = [uint8; N]" template=(comptime N: usize) value=FixedArray<uint8, N>
/// @type.symbol symbol=Slots.N source="comptime N: usize" type=N
/// @resolution.name source=N target=Slots.N

type Bytes = Slots<16>;
/// @type.symbol symbol=Bytes source="type Bytes = Slots<16>" type=FixedArray<uint8, 16>
/// @definition.type symbol=Bytes source="type Bytes = Slots<16>" value=FixedArray<uint8, 16>
/// @resolution.name source=Slots target=Slots

declare const bytes: Bytes;
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 16>
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @resolution.name source=Bytes target=Bytes
"#,
    );
}

#[test]
fn test_parameter_position_transparent_constraint_induces_generic() {
    let session = TestSession::single(
        r#"
type Printable = { print(): string };

function print(value: Printable): string {
    return value.print();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Printable = { print(): string };

function print(value: { print: () => string }): string {
    return value.print();
}

=== checked ===
type Printable = { print(): string };
/// @type.symbol symbol=Printable source="type Printable = { print(): string }" type={ print(): string }
/// @definition.type symbol=Printable source="type Printable = { print(): string }" value={ print(): string }

function print(value: Printable): string {
/// @type.symbol symbol=print type=(Printable) => string
/// @type.symbol symbol=print.value source="value: Printable" type={ print(): string }
/// @resolution.name source=Printable target=Printable

    return value.print();
    /// @type.node source=value type={ print(): string }
    /// @type.node source=value.print type=() => string
    /// @type.node source=value.print() type=string
    /// @resolution.name source=value target=print.value
    /// @resolution.member source=value.print receiver={ print(): string } type=() => string kind=field target_receiver={ print(): string } key=print target_type=() => string
    /// @resolution.call source=value.print() parameters=() return=string kind=expression target=expression
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=print.value
    /// @resolution.access source=value.print root=print.value keys=[print]

}
"#,
    );
}

#[test]
fn test_declared_transparent_return_preserves_declared_type() {
    let session = TestSession::single(
        r#"
type Shape = Circle | Rectangle;

struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

function makeCircle(): Shape {
    return Circle { radius: 1.0 };
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Shape = Circle | Rectangle;

struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

function makeCircle(): Shape {
    return Circle { radius: 1.0 } as Circle | Rectangle;
}

=== checked ===
type Shape = Circle | Rectangle;
/// @type.symbol symbol=Shape source="type Shape = Circle | Rectangle" type=Circle | Rectangle
/// @definition.type symbol=Shape source="type Shape = Circle | Rectangle" value=Circle | Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.field symbol=Circle.radius source="radius: float64" key=radius type=float64

    radius: float64;
    /// @type.symbol symbol=Circle.radius source="radius: float64" type=float64

}

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.height source="height: float64" key=height type=float64
/// @definition.field symbol=Rectangle.width source="width: float64" key=width type=float64

    width: float64;
    /// @type.symbol symbol=Rectangle.width source="width: float64" type=float64

    height: float64;
    /// @type.symbol symbol=Rectangle.height source="height: float64" type=float64

}

function makeCircle(): Shape {
/// @type.symbol symbol=makeCircle type=() => Shape
/// @resolution.name source=Shape target=Shape

    return Circle { radius: 1.0 };
    /// @resolution.name source=Circle target=Circle

}
"#,
    );
}
