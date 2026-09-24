use crate::tests::{DirRows, TestSession};

#[test]
fn test_substitute_a_const_argument_into_a_fixed_array_alias() {
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

declare const bytes: Bytes;

=== dir ===
type Slots<const N: usize> = [uint8; N];
/// @generic.template symbol=Slots parameters=(const N: usize)
/// @type.symbol symbol=Slots source="type Slots<const N: usize> = [uint8; N]" type=FixedArray<uint8, N>
/// @definition.type symbol=Slots source="type Slots<const N: usize> = [uint8; N]" template=(const N: usize) value=FixedArray<uint8, N>
/// @type.symbol symbol=Slots.N source="const N: usize" type=N
/// @resolution.name source=N target=Slots.N

type Bytes = Slots<16>;
/// @type.symbol symbol=Bytes source="type Bytes = Slots<16>" type=FixedArray<uint8, 16>
/// @generic.instance id=Slots<16> template=Slots arguments=(16)
/// @definition.type symbol=Bytes source="type Bytes = Slots<16>" value=Slots<16>
/// @resolution.name source=Slots target=Slots

declare const bytes: Bytes;
/// @type.symbol symbol=bytes source=bytes type=Bytes
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
/// @generic.instance id="Result<T#2, E#2>" template=Result arguments=(T#2, E#2)
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
        /// @generic.instantiation id="Result<T#2, E#2>" template=Result arguments=(T#2, E#2) owner=ok
        /// @resolution.name source=value target=ok.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=ok.value

    }
}

const result = Result<int32, string>.ok(42);
/// @type.symbol symbol=result source=result type=Result<int32, string>
/// @resolution.pattern source=result kind=binding target=result
/// @generic.instance id="Result<int32, string>" template=Result arguments=(int32, string)
/// @resolution.name source="Result<int32, string>" target=Result
/// @resolution.name source=Result target=Result
/// @resolution.member source="Result<int32, string>.ok" receiver=Result<int32, string> type=(int32) => Result<int32, string> kind=symbol target_receiver=Result<int32, string> target=ok
/// @resolution.call source="Result<int32, string>.ok(42)" parameters=(int32) arguments=(provided(42) as int32) return=Result<int32, string> kind=symbol target=ok instance="Result<int32, string>.<extension#1>.ok"
/// @generic.instantiation id="ok<int32, string>" template=ok arguments=(int32, string)
/// @generic.instance id="ok<int32, string>" template=ok arguments=(int32, string)
"#,
    );
}

#[test]
fn test_call_a_method_through_a_structural_alias_parameter() {
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

function print(value: Printable): string {
    return value.print();
}

=== dir ===
type Printable = { print(): string };
/// @type.symbol symbol=Printable source="type Printable = { print(): string }" type={ print(): string }
/// @definition.type symbol=Printable source="type Printable = { print(): string }" value={ print(): string }

function print(value: Printable): string {
/// @type.symbol symbol=print type=(Printable) => string
/// @type.symbol symbol=print.value source="value: Printable" type=Printable
/// @resolution.name source=Printable target=Printable

    return value.print();
    /// @type.node source=value type=Printable
    /// @type.node source=value.print type=() => string
    /// @type.node source=value.print() type=string
    /// @resolution.name source=value target=print.value
    /// @resolution.member source=value.print receiver=Printable type=() => string kind=field target_receiver=Printable key=print target_type=() => string
    /// @resolution.call source=value.print() parameters=() return=string kind=expression target=expression
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=print.value
    /// @resolution.place source=value.print placement="local" lifetime="managed" access="mutable"
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
    return Circle { radius: 1.0 } as Shape;
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

#[test]
fn test_infer_a_bounded_parameter_from_literal_elements() {
    let session = TestSession::single(
        r#"
declare function first<T: readonly unknown[]>(items: T[]): T;
declare function box<T: unknown>(value: T): T;

const row = first([[1, 2]]);
const boxed = box(1);
const mixed = first([["a", 1]]);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function first<T: readonly unknown[]>(items: T[]): T;
declare function box<T: unknown>(value: T): T;

const row: int64[] = first<int64[]>([[1, 2]]);
const boxed: int64 = box<int64>(1);
const mixed: (string | int64)[] = first<(string | int64)[]>([
    ["a" as string | int64, 1 as string | int64],
]);

=== dir ===
declare function first<T: readonly unknown[]>(items: T[]): T;
/// @generic.template symbol=first parameters=(T#1: readonly unknown[])
/// @type.symbol symbol=first source="declare function first<T: readonly unknown[]>(items: T[]): T" type=<T#1: readonly unknown[]>(T#1[]) => T#1
/// @type.symbol symbol=first.T source="T: readonly unknown[]" type=T#1
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

declare function box<T: unknown>(value: T): T;
/// @generic.template symbol=box parameters=(T#2: unknown)
/// @type.symbol symbol=box source="declare function box<T: unknown>(value: T): T" type=<T#2: unknown>(T#2) => T#2
/// @type.symbol symbol=box.T source="T: unknown" type=T#2
/// @resolution.name source=T target=box.T
/// @resolution.name source=T target=box.T

const row = first([[1, 2]]);
/// @type.symbol symbol=row source=row type=int64[]
/// @resolution.pattern source=row kind=binding target=row
/// @resolution.name source=first target=first
/// @resolution.call source="first([[1, 2]])" parameters=(int64[][]) arguments=(provided([[1, 2]]) as int64[][]) return=int64[] kind=symbol target=first instance=first<int64[]>
/// @generic.instantiation id=first<int64[]> template=first arguments=(int64[])
/// @resolution.call source=[[1, 2]] parameters=(^Slice<int64[]>) arguments=(rest(provided([1, 2]) as int64[]) as int64[]) return=int64[][] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64[]>
/// @generic.instantiation id=arrayFromOwnedSlice<int64[]> template=arrayFromOwnedSlice arguments=(int64[])
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)

const boxed = box(1);
/// @type.symbol symbol=boxed source=boxed type=int64
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=box target=box
/// @resolution.call source=box(1) parameters=(int64) arguments=(provided(1) as int64) return=int64 kind=symbol target=box instance=box<int64>
/// @generic.instantiation id=box<int64> template=box arguments=(int64)

const mixed = first([["a", 1]]);
/// @type.symbol symbol=mixed source=mixed type=string | int64[]
/// @resolution.pattern source=mixed kind=binding target=mixed
/// @resolution.name source=first target=first
/// @resolution.call source="first([[\"a\", 1]])" parameters=(string | int64[][]) arguments=(provided([["a", 1]]) as string | int64[][]) return=string | int64[] kind=symbol target=first instance="first<string | int64[]>"
/// @generic.instantiation id="first<string | int64[]>" template=first arguments=(string | int64[])
/// @resolution.call source=[["a", 1]] parameters=(^Slice<string | int64[]>) arguments=(rest(provided(["a", 1]) as string | int64[]) as string | int64[]) return=string | int64[][] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<string | int64[]>"
/// @generic.instantiation id="arrayFromOwnedSlice<string | int64[]>" template=arrayFromOwnedSlice arguments=(string | int64[])
/// @resolution.call source=["a", 1] parameters=(^Slice<string | int64>) arguments=(rest(provided("a") as string | int64, provided(1) as string | int64) as string | int64) return=string | int64[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<string | int64>"
/// @generic.instantiation id="arrayFromOwnedSlice<string | int64>" template=arrayFromOwnedSlice arguments=(string | int64)
"#,
        r#"

"#,
    );
}

#[test]
fn test_flow_an_expectation_into_literal_elements_through_a_generic_call() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;
declare function first<T>(values?: T[]): T;

const one: (int32,) = id((1,));
const items: int32[] = id([1, 2]);
const plain = id((1,));
const optional = first([1, 2]);
const expected: int32 = first([1, 2]);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;
declare function first<T>(values?: T[]): T;

const one: (int32,) = id<(int32,)>((1,));
const items: int32[] = id<int32[]>([1, 2]);
const plain: (int64,) = id<(int64,)>((1,));
const optional: int64 = first<int64>([1, 2] as int64[] | undefined);
const expected: int32 = first<int32>([1, 2] as int32[] | undefined);

=== dir ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T#1)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T#1>(T#1) => T#1
/// @type.symbol symbol=id.T source=T type=T#1
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

declare function first<T>(values?: T[]): T;
/// @generic.template symbol=first parameters=(T#2)
/// @type.symbol symbol=first source="declare function first<T>(values?: T[]): T" type=<T#2>(T#2[] | undefined?) => T#2
/// @type.symbol symbol=first.T source=T type=T#2
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

const one: (int32,) = id((1,));
/// @type.symbol symbol=one source=one type=(int32,)
/// @resolution.pattern source=one kind=binding target=one
/// @resolution.name source=id target=id
/// @resolution.call source=id((1,)) parameters=((int32,)) arguments=(provided((1,)) as (int32,)) return=(int32,) kind=symbol target=id instance=id<(int32,)>
/// @generic.instantiation id=id<(int32,)> template=id arguments=((int32,))

const items: int32[] = id([1, 2]);
/// @type.symbol symbol=items source=items type=int32[]
/// @resolution.pattern source=items kind=binding target=items
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(int32[]) arguments=(provided([1, 2]) as int32[]) return=int32[] kind=symbol target=id instance=id<int32[]>
/// @generic.instantiation id=id<int32[]> template=id arguments=(int32[])
/// @resolution.call source=[1, 2] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

const plain = id((1,));
/// @type.symbol symbol=plain source=plain type=(int64,)
/// @resolution.pattern source=plain kind=binding target=plain
/// @resolution.name source=id target=id
/// @resolution.call source=id((1,)) parameters=((int64,)) arguments=(provided((1,)) as (int64,)) return=(int64,) kind=symbol target=id instance=id<(int64,)>
/// @generic.instantiation id=id<(int64,)> template=id arguments=((int64,))

const optional = first([1, 2]);
/// @type.symbol symbol=optional source=optional type=int64
/// @resolution.pattern source=optional kind=binding target=optional
/// @resolution.name source=first target=first
/// @resolution.call source="first([1, 2])" parameters=(int64[] | undefined) arguments=(provided([1, 2]) as int64[] | undefined) return=int64 kind=symbol target=first instance=first<int64>
/// @generic.instantiation id=first<int64> template=first arguments=(int64)
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)

const expected: int32 = first([1, 2]);
/// @type.symbol symbol=expected source=expected type=int32
/// @resolution.pattern source=expected kind=binding target=expected
/// @resolution.name source=first target=first
/// @resolution.call source="first([1, 2])" parameters=(int32[] | undefined) arguments=(provided([1, 2]) as int32[] | undefined) return=int32 kind=symbol target=first instance=first<int32>
/// @generic.instantiation id=first<int32> template=first arguments=(int32)
/// @resolution.call source=[1, 2] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
"#,
        r#"

"#,
    );
}
