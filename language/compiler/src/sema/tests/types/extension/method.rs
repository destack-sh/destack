use crate::tests::{DirRows, TestSession};

#[test]
fn test_inherent_extension_method_resolves_on_receiver() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

extension of Point {
    sum(&readonly this): int32 {
        return this.x + this.y;
    }
}

declare const point: Point;
const value = point.sum();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

extension of Point {
    sum(&readonly this): int32 {
        return this.x + this.y;
    }
}

declare const point: Point;
const value: int32 = point.sum<"static">();

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

extension of Point {
/// @definition.extension symbol=<module>#2 form=local target=Point
/// @definition.method symbol=sum slot=sum type=<sum.'a>(this: &sum.'a readonly Point) => int32
/// @resolution.name source=Point target=Point

    sum(&readonly this): int32 {
    /// @generic.template symbol=sum parameters=('a)
    /// @type.symbol symbol=sum type=<sum.'a>(this: &sum.'a readonly Point) => int32
    /// @type.symbol symbol=sum.this source="&readonly this" type=&sum.'a readonly Point

        return this.x + this.y;
        /// @type.node source="this.x + this.y" type=int32
        /// @type.node source=this type=&sum.'a readonly Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=&sum.'a readonly Point type=int32 kind=field target_receiver=&sum.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.operator source="this.x + this.y" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), this.y as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&sum.'a readonly Point
        /// @resolution.place source=this placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @type.node source=this type=&sum.'a readonly Point
        /// @type.node source=this.y type=int32
        /// @resolution.member source=this.y receiver=&sum.'a readonly Point type=int32 kind=field target_receiver=&sum.'a readonly Point key=y target=Point.y target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&sum.'a readonly Point
        /// @resolution.place source=this placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.y placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.y root=this keys=[y]

    }
}

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

const value = point.sum();
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=point type=Point
/// @type.node source=point.sum type=<sum.'a>(this: &sum.'a readonly Point) => int32
/// @type.node source=point.sum() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.sum receiver=Point type=<sum.'a>(this: &sum.'a readonly Point) => int32 kind=symbol target_receiver=Point target=sum
/// @resolution.call source=point.sum() parameters=() return=int32 regions=("static" & "local") kind=symbol target=sum receiver=Point adjustments=(borrow(&'static readonly Point)) instance="Point.<extension#1>.sum<\"static\" & \"local\">"
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @generic.instantiation id="sum<\"static\" & \"local\">" template=sum arguments=("static" & "local")
/// @generic.instance id="sum<\"bound0\" & \"local\">" template=sum arguments=("bound0" & "local")
"#);
}

#[test]
fn test_missing_extension_method_reports_missing_member() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

extension of Point {
    sum(): int32 {
        return this.x + this.y;
    }
}

declare const point: Point;
point.length();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

extension of Point {
    sum(): int32 {
        return this.x + this.y;
    }
}

declare const point: Point;
point.length();

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

extension of Point {
/// @definition.extension symbol=<module>#2 form=local target=Point
/// @definition.method symbol=sum slot=sum type=<sum.'a>(this: &sum.'a readonly Point) => int32
/// @resolution.name source=Point target=Point

    sum(): int32 {
    /// @generic.template symbol=sum parameters=('a)
    /// @type.symbol symbol=sum type=<sum.'a>(this: &sum.'a readonly Point) => int32
    /// @type.symbol symbol=sum.this type=&sum.'a readonly Point

        return this.x + this.y;
        /// @type.node source="this.x + this.y" type=int32
        /// @type.node source=this type=&sum.'a readonly Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=&sum.'a readonly Point type=int32 kind=field target_receiver=&sum.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.operator source="this.x + this.y" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), this.y as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&sum.'a readonly Point
        /// @resolution.place source=this placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @type.node source=this type=&sum.'a readonly Point
        /// @type.node source=this.y type=int32
        /// @resolution.member source=this.y receiver=&sum.'a readonly Point type=int32 kind=field target_receiver=&sum.'a readonly Point key=y target=Point.y target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&sum.'a readonly Point
        /// @resolution.place source=this placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.y placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.y root=this keys=[y]

    }
}

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

point.length();
/// @type.node source=point type=Point
/// @type.node source=point.length type=<error>
/// @type.node source=point.length() type=<error>
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @resolution.rejected source=point.length
/// @resolution.rejected source=point.length()
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'length' does not exist on type 'Point'"
/// @diagnostic.label line=14 column=7 span="length" line_source="point.length();"
"#,
    );
}

#[test]
fn test_extension_getter_selects_through_a_borrowed_receiver() {
    let session = TestSession::single(
        r#"
struct Slice<in out T> {
    length: usize;
}

extension<T> of Slice<T> {
    get size(this: &readonly Slice<T>): usize {
        this.length
    }

    first(this: &readonly Slice<T>): usize {
        this.size
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Slice<in out T> {
    length: usize;
}

extension<T> of Slice<T> {
    get size(this: &readonly Slice<T>): usize {
        this.length
    }

    first(this: &readonly Slice<T>): usize {
        this.size
    }
}

=== dir ===
struct Slice<in out T> {
/// @generic.template symbol=Slice parameters=(in out T#1)
/// @type.symbol symbol=Slice type=Slice
/// @definition.struct symbol=Slice template=(in out T#1)
/// @definition.field symbol=Slice.length source="length: usize" key=length type=usize
/// @type.symbol symbol=Slice.T source="in out T" type=T#1

    length: usize;
    /// @type.symbol symbol=Slice.length source="length: usize" type=usize

}

extension<T> of Slice<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Slice<T#2>
/// @definition.method symbol=first slot=first type=<first.'a>(this: &first.'a readonly Slice<T#2>) => usize
/// @definition.method symbol=size slot=size role=getter type=<size.'a>(this: &size.'a readonly Slice<T#2>) => usize
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Slice target=Slice
/// @resolution.name source=T target=T

    get size(this: &readonly Slice<T>): usize {
    /// @generic.template symbol=size parent=template#1 parameters=('a)
    /// @type.symbol symbol=size type=<size.'a>(this: &size.'a readonly Slice<T#2>) => usize
    /// @type.symbol symbol=size.this source="this: &readonly Slice<T>" type=&size.'a readonly Slice<T#2>
    /// @resolution.name source=Slice target=Slice
    /// @resolution.name source=T target=T

        this.length
        /// @resolution.member source=this.length receiver=&size.'a readonly Slice<T#2> type=usize kind=field target_receiver=&size.'a readonly Slice<T#2> key=length target=Slice.length target_type=usize
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&size.'a readonly Slice<T#2>
        /// @resolution.place source=this placement=size.'a lifetime=size.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.length placement=size.'a lifetime=size.'a access="readonly"
        /// @resolution.access source=this.length root=this keys=[length]

    }

    first(this: &readonly Slice<T>): usize {
    /// @generic.template symbol=first parent=template#1 parameters=('a)
    /// @type.symbol symbol=first type=<first.'a>(this: &first.'a readonly Slice<T#2>) => usize
    /// @type.symbol symbol=first.this source="this: &readonly Slice<T>" type=&first.'a readonly Slice<T#2>
    /// @resolution.name source=Slice target=Slice
    /// @resolution.name source=T target=T

        this.size
        /// @resolution.member source=this.size receiver=&first.'a readonly Slice<T#2> type=usize kind=call target="size(parameters=(), arguments=(), return=usize, regions=(first.'a))"
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&first.'a readonly Slice<T#2>
        /// @resolution.place source=this placement=first.'a lifetime=first.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="size<T#2, first.'a>" template=size arguments=(T#2, first.'a) owner=first

    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_a_mutable_receiver_method_on_a_readonly_borrow() {
    let session = TestSession::single(
        r#"
struct Buffer {
    length: usize;
}

extension of Buffer {
    grow(this: &Buffer): void {}

    peek(this: &readonly Buffer): void {
        this.grow()
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Buffer {
    length: usize;
}

extension of Buffer {
    grow(this: &Buffer): void {}

    peek(this: &readonly Buffer): void {
        this.grow<'a>();
    }
}

=== dir ===
struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.length source="length: usize" key=length type=usize

    length: usize;
    /// @type.symbol symbol=Buffer.length source="length: usize" type=usize

}

extension of Buffer {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.method symbol=grow source="grow(this: &Buffer): void {}" slot=grow type=<grow.'a>(this: &grow.'a Buffer) => void
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: &peek.'a readonly Buffer) => void
/// @resolution.name source=Buffer target=Buffer

    grow(this: &Buffer): void {}
    /// @generic.template symbol=grow parameters=('a)
    /// @type.symbol symbol=grow source="grow(this: &Buffer): void {}" type=<grow.'a>(this: &grow.'a Buffer) => void
    /// @type.symbol symbol=grow.this source="this: &Buffer" type=&grow.'a Buffer
    /// @resolution.name source=Buffer target=Buffer

    peek(this: &readonly Buffer): void {
    /// @generic.template symbol=peek parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: &peek.'a readonly Buffer) => void
    /// @type.symbol symbol=peek.this source="this: &readonly Buffer" type=&peek.'a readonly Buffer
    /// @resolution.name source=Buffer target=Buffer

        this.grow()
        /// @resolution.member source=this.grow receiver=&peek.'a readonly Buffer type=<grow.'a>(this: &grow.'a Buffer) => void kind=symbol target_receiver=&peek.'a readonly Buffer target=grow
        /// @resolution.call source=this.grow() parameters=() return=void regions=(peek.'a) kind=symbol target=grow receiver=&peek.'a readonly Buffer instance=Buffer.<extension#1>.grow<peek.'a>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&peek.'a readonly Buffer
        /// @resolution.place source=this placement=peek.'a lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=grow<peek.'a> template=grow arguments=(peek.'a)

    }
}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'a readonly Buffer' is not assignable to the method's 'this' type '&'a Buffer'"
/// @diagnostic.label line=10 column=9 span="this.grow()" line_source="this.grow()"
"#,
    );
}

#[test]
fn test_blanket_extension_method_resolves_on_primitive_receiver() {
    let session = TestSession::single(
        r#"
interface Scalar {}

extension Doubling<T: Scalar> of T {
    double(this): T {
        this
    }
}

const value = 1.double();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Scalar {}

extension Doubling<T: Scalar> of T {
    double(this): T {
        this
    }
}

const value: 1 = (1).double<1>();

=== dir ===
interface Scalar {}
/// @generic.template symbol=Scalar parameters=(this: Scalar)
/// @type.symbol symbol=Scalar source="interface Scalar {}" type=Scalar
/// @definition.interface symbol=Scalar source="interface Scalar {}" template=(this: Scalar)
/// @definition.where symbol=Scalar source="interface Scalar {}" relation=satisfies left=this right=Scalar

extension Doubling<T: Scalar> of T {
/// @generic.template symbol=Doubling parameters=(T: Scalar)
/// @definition.extension symbol=Doubling form=local target=T
/// @definition.method symbol=Doubling.double slot=double type=(this: T) => T
/// @type.symbol symbol=Doubling.T source="T: Scalar" type=T
/// @resolution.name source=Scalar target=Scalar
/// @resolution.name source=T target=Doubling.T

    double(this): T {
    /// @type.symbol symbol=Doubling.double type=(this: T) => T
    /// @type.symbol symbol=Doubling.double.this source=this type=T
    /// @resolution.name source=T target=Doubling.T

        this
        /// @type.node source=this type=T
        /// @resolution.receiver source=this kind=this declaration=Doubling type=T
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

    }
}

const value = 1.double();
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
/// @type.node source=1.double type=(this: 1) => 1
/// @type.node source=1.double() type=1
/// @resolution.member source=1.double receiver=1 type=(this: 1) => 1 kind=symbol target_receiver=1 target=Doubling.double
/// @resolution.call source=1.double() parameters=() return=1 kind=symbol target=Doubling.double receiver=1 instance=Doubling<1>.double
/// @generic.instantiation id=Doubling.double<1> template=Doubling.double arguments=(1)
"#,
        r#"
"#,
    );
}
