use crate::tests::{DirRows, TestSession};

#[test]
fn test_value_generic_argument_still_works_when_parameter_is_static() {
    let session = TestSession::single(
        r#"
type Slots<const N: usize> = [uint8; N];
type Bytes = Slots<16>;

declare const bytes: Bytes;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
type Slots<const N: usize> = [uint8; N];
type Bytes = Slots<16>;

declare const bytes: [uint8; 16];

=== dir ===
type Slots<const N: usize> = [uint8; N];
/// @generic.template symbol=Slots parameters=(const N: usize)
/// @type.symbol symbol=Slots source="type Slots<const N: usize> = [uint8; N]" type=FixedArray<uint8, N>
/// @definition.type symbol=Slots source="type Slots<const N: usize> = [uint8; N]" template=(const N: usize) value=FixedArray<uint8, N>
/// @type.symbol symbol=Slots.N source="const N: usize" type=N
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
fn test_applied_receiver_arguments_bind_static_extension_call() {
    let session = TestSession::single(
        r#"
newtype Result<T, E> = T | E;

extension<T, E> of Result<T, E> {
    static ok(value: T): Result<T, E> {
        Result(value)
    }
}

const result = Result<int32, string>.ok(42);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Result<out T, out E> = T | E;

extension<T, E> of Result<T, E> {
    static ok(value: T): Result<T, E> {
        Result(value)
    }
}

const result: Result<int32, string> = Result<int32, string>.ok<int32, string>(42);

=== dir ===
newtype Result<T, E> = T | E;
/// @generic.template symbol=Result parameters=(out T#1, out E#1)
/// @type.symbol symbol=Result source="newtype Result<T, E> = T | E" type=Result
/// @definition.newtype symbol=Result source="newtype Result<T, E> = T | E" template=(out T#1, out E#1) backing=T#1 | E#1 constructors=[<T#1, E#1>(T#1) => Result<T#1, E#1>, <T#1, E#1>(E#1) => Result<T#1, E#1>, <T#1, E#1>(T#1 | E#1) => Result<T#1, E#1>]
/// @type.symbol symbol=Result.T source=T type=T#1
/// @type.symbol symbol=Result.E source=E type=E#1
/// @resolution.name source=T target=Result.T
/// @resolution.name source=E target=Result.E

extension<T, E> of Result<T, E> {
/// @generic.template symbol=<module>#2 parameters=(T#2, E#2)
/// @definition.extension symbol=<module>#2 form=local target=Result<T#2, E#2>
/// @definition.method symbol=ok slot=ok static=true type=(T#2) => Result<T#2, E#2>
/// @type.symbol symbol=T source=T type=T#2
/// @type.symbol symbol=E source=E type=E#2
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=T
/// @resolution.name source=E target=E

    static ok(value: T): Result<T, E> {
    /// @type.symbol symbol=ok type=(T#2) => Result<T#2, E#2>
    /// @type.symbol symbol=ok.value source="value: T" type=T#2
    /// @resolution.name source=T target=T
    /// @resolution.name source=Result target=Result
    /// @resolution.name source=T target=T
    /// @resolution.name source=E target=E

        Result(value)
        /// @resolution.name source=Result target=Result
        /// @resolution.construct source=Result(value) parameters=(T#2) arguments=(provided(value) as T#2) return=Result<T#2, E#2> kind=newtype target=Result backing=T#2 instance="Result<T#2, E#2>"
        /// @generic.instantiation id="Result<T#2, E#2>" template=Result arguments=(T#2, E#2) owner=<module>#2
        /// @resolution.name source=value target=ok.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=ok.value

    }
}

const result = Result<int32, string>.ok(42);
/// @type.symbol symbol=result source=result type=Result<int32, string>
/// @resolution.pattern source=result kind=binding target=result
/// @generic.instance id="Result<int32, string>" template=Result arguments=(int32, string)
/// @resolution.name source=Result target=Result
/// @resolution.member source="Result<int32, string>.ok" receiver=Result<int32, string> type=(int32) => Result<int32, string> kind=symbol target_receiver=Result<int32, string> target=ok
/// @resolution.call source="Result<int32, string>.ok(42)" parameters=(int32) arguments=(provided(42) as int32) return=Result<int32, string> kind=symbol target=ok instance="Result<int32, string>.<extension#1>.ok"
/// @resolution.function source="Result<int32, string>" type=Result<int32, string> target=Result instance="Result<int32, string>"
/// @generic.instantiation id="Result<int32, string>" template=Result arguments=(int32, string)
/// @generic.instantiation id="ok<int32, string>" template=ok arguments=(int32, string)
/// @generic.instantiation id="ok<int32, string>" template=ok arguments=(int32, string)
/// @generic.instance id="ok<int32, string>" template=ok arguments=(int32, string)
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Printable = { print(): string };

function print(value: { print: () => string }): string {
    return value.print();
}

=== dir ===
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

    session.assert_dir(
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

=== dir ===
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
