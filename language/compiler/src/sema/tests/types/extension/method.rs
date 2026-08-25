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
        "main.ds",
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
const value: int32 = point.sum<"constant">();

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
/// @definition.method symbol=sum slot=sum type=<sum.'a, sum.P1: Place>(this: Borrowed<Point, sum.'a & sum.P1, "readonly">) => int32
/// @resolution.name source=Point target=Point

    sum(&readonly this): int32 {
    /// @generic.template symbol=sum parameters=('a, P1: Place)
    /// @type.symbol symbol=sum type=<sum.'a, sum.P1: Place>(this: Borrowed<Point, sum.'a & sum.P1, "readonly">) => int32
    /// @type.symbol symbol=sum.this source="&readonly this" type=Borrowed<this, sum.'a & sum.P1, "readonly">

        return this.x + this.y;
        /// @type.node source="this.x + this.y" type=int32
        /// @type.node source=this type=Borrowed<Point, sum.'a & sum.P1, "readonly">
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=Borrowed<Point, sum.'a & sum.P1, "readonly"> type=int32 kind=field target_receiver=Borrowed<Point, sum.'a & sum.P1, "readonly"> key=x target=Point.x target_type=int32
        /// @resolution.operator source="this.x + this.y" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), this.y as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Point, sum.'a & sum.P1, "readonly">
        /// @resolution.place source=this placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @type.node source=this type=Borrowed<Point, sum.'a & sum.P1, "readonly">
        /// @type.node source=this.y type=int32
        /// @resolution.member source=this.y receiver=Borrowed<Point, sum.'a & sum.P1, "readonly"> type=int32 kind=field target_receiver=Borrowed<Point, sum.'a & sum.P1, "readonly"> key=y target=Point.y target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Point, sum.'a & sum.P1, "readonly">
        /// @resolution.place source=this placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.y placement=sum.P1 lifetime=sum.'a access="readonly"
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
/// @type.node source=point.sum type=<sum.'a, sum.P1: Place>(this: Borrowed<Point, sum.'a & sum.P1, "readonly">) => int32
/// @type.node source=point.sum() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.sum receiver=Point type=<sum.'a, sum.P1: Place>(this: Borrowed<Point, sum.'a & sum.P1, "readonly">) => int32 kind=symbol target_receiver=Point target=sum
/// @resolution.call source=point.sum() parameters=() return=int32 kind=symbol target=sum receiver=Point adjustments=(borrow(&'static readonly constant Point)) instance="Point.<extension#1>.sum<\"constant\">"
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @generic.instantiation id="sum<\"constant\">" template=sum arguments=("constant")
/// @generic.instance id="sum<\"constant\">" template=sum arguments=("constant")
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
        "main.ds",
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
/// @definition.method symbol=sum slot=sum type=<sum.'a, sum.P1: Place>(this: Borrowed<this, sum.'a & sum.P1, "readonly">) => int32
/// @resolution.name source=Point target=Point

    sum(): int32 {
    /// @generic.template symbol=sum parameters=('a, P1: Place)
    /// @type.symbol symbol=sum type=<sum.'a, sum.P1: Place>(this: Borrowed<this, sum.'a & sum.P1, "readonly">) => int32

        return this.x + this.y;
        /// @type.node source="this.x + this.y" type=int32
        /// @type.node source=this type=Borrowed<Point, sum.'a & sum.P1, "readonly">
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=Borrowed<Point, sum.'a & sum.P1, "readonly"> type=int32 kind=field target_receiver=Borrowed<Point, sum.'a & sum.P1, "readonly"> key=x target=Point.x target_type=int32
        /// @resolution.operator source="this.x + this.y" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), this.y as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Point, sum.'a & sum.P1, "readonly">
        /// @resolution.place source=this placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @type.node source=this type=Borrowed<Point, sum.'a & sum.P1, "readonly">
        /// @type.node source=this.y type=int32
        /// @resolution.member source=this.y receiver=Borrowed<Point, sum.'a & sum.P1, "readonly"> type=int32 kind=field target_receiver=Borrowed<Point, sum.'a & sum.P1, "readonly"> key=y target=Point.y target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Point, sum.'a & sum.P1, "readonly">
        /// @resolution.place source=this placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.y placement=sum.P1 lifetime=sum.'a access="readonly"
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
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
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
        "main.ds",
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
/// @definition.method symbol=first slot=first type=<first.'a, first.P1: Place>(this: Borrowed<Slice<T#2>, first.'a & first.P1, "readonly">) => usize
/// @definition.method symbol=size slot=size role=getter type=<size.'a, size.P1: Place>(this: Borrowed<Slice<T#2>, size.'a & size.P1, "readonly">) => usize
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Slice target=Slice
/// @resolution.name source=T target=T

    get size(this: &readonly Slice<T>): usize {
    /// @generic.template symbol=size parent=template#1 parameters=('a, P1: Place)
    /// @type.symbol symbol=size type=<size.'a, size.P1: Place>(this: Borrowed<Slice<T#2>, size.'a & size.P1, "readonly">) => usize
    /// @type.symbol symbol=size.this source="this: &readonly Slice<T>" type=Borrowed<Slice<T#2>, size.'a & size.P1, "readonly">
    /// @resolution.name source=Slice target=Slice
    /// @resolution.name source=T target=T

        this.length
        /// @resolution.member source=this.length receiver=Borrowed<Slice<T#2>, size.'a & size.P1, "readonly"> type=usize kind=field target_receiver=Borrowed<Slice<T#2>, size.'a & size.P1, "readonly"> key=length target=Slice.length target_type=usize
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Slice<T#2>, size.'a & size.P1, "readonly">
        /// @resolution.place source=this placement=size.P1 lifetime=size.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.length placement=size.P1 lifetime=size.'a access="readonly"
        /// @resolution.access source=this.length root=this keys=[length]

    }

    first(this: &readonly Slice<T>): usize {
    /// @generic.template symbol=first parent=template#1 parameters=('a, P1: Place)
    /// @type.symbol symbol=first type=<first.'a, first.P1: Place>(this: Borrowed<Slice<T#2>, first.'a & first.P1, "readonly">) => usize
    /// @type.symbol symbol=first.this source="this: &readonly Slice<T>" type=Borrowed<Slice<T#2>, first.'a & first.P1, "readonly">
    /// @resolution.name source=Slice target=Slice
    /// @resolution.name source=T target=T

        this.size
        /// @resolution.member source=this.size receiver=Borrowed<Slice<T#2>, first.'a & first.P1, "readonly"> type=usize kind=call target="size(parameters=(), arguments=(), return=usize)"
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Slice<T#2>, first.'a & first.P1, "readonly">
        /// @resolution.place source=this placement=first.P1 lifetime=first.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="size<T#2, first.P1>" template=size arguments=(T#2, first.P1) owner=first

    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_readonly_borrow_rejects_an_exclusive_receiver_method() {
    let session = TestSession::single(
        r#"
struct Buffer {
    length: usize;
}

extension of Buffer {
    grow(this: &exclusive Buffer): void {}

    peek(this: &readonly Buffer): void {
        this.grow()
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Buffer {
    length: usize;
}

extension of Buffer {
    grow(this: &exclusive Buffer): void {}

    peek(this: &readonly Buffer): void {
        this.grow<P1>();
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
/// @definition.method symbol=grow source="grow(this: &exclusive Buffer): void {}" slot=grow type=<grow.'a, grow.P1: Place>(this: Borrowed<Buffer, grow.'a & grow.P1, "exclusive">) => void
/// @definition.method symbol=peek slot=peek type=<peek.'a, peek.P1: Place>(this: Borrowed<Buffer, peek.'a & peek.P1, "readonly">) => void
/// @resolution.name source=Buffer target=Buffer

    grow(this: &exclusive Buffer): void {}
    /// @generic.template symbol=grow parameters=('a, P1: Place)
    /// @type.symbol symbol=grow source="grow(this: &exclusive Buffer): void {}" type=<grow.'a, grow.P1: Place>(this: Borrowed<Buffer, grow.'a & grow.P1, "exclusive">) => void
    /// @type.symbol symbol=grow.this source="this: &exclusive Buffer" type=Borrowed<Buffer, grow.'a & grow.P1, "exclusive">
    /// @resolution.name source=Buffer target=Buffer

    peek(this: &readonly Buffer): void {
    /// @generic.template symbol=peek parameters=('a, P1: Place)
    /// @type.symbol symbol=peek type=<peek.'a, peek.P1: Place>(this: Borrowed<Buffer, peek.'a & peek.P1, "readonly">) => void
    /// @type.symbol symbol=peek.this source="this: &readonly Buffer" type=Borrowed<Buffer, peek.'a & peek.P1, "readonly">
    /// @resolution.name source=Buffer target=Buffer

        this.grow()
        /// @resolution.member source=this.grow receiver=Borrowed<Buffer, peek.'a & peek.P1, "readonly"> type=<grow.'a, grow.P1: Place>(this: Borrowed<Buffer, grow.'a & grow.P1, "exclusive">) => void kind=symbol target_receiver=Borrowed<Buffer, peek.'a & peek.P1, "readonly"> target=grow
        /// @resolution.call source=this.grow() parameters=() return=void kind=symbol target=grow receiver=Borrowed<Buffer, peek.'a & peek.P1, "readonly"> instance=Buffer.<extension#1>.grow<peek.P1>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Buffer, peek.'a & peek.P1, "readonly">
        /// @resolution.place source=this placement=peek.P1 lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=grow<peek.P1> template=grow arguments=(peek.P1)

    }
}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'a readonly Buffer' is not assignable to the method's 'this' type '&exclusive Buffer'"
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
        "main.ds",
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
/// @type.symbol symbol=Scalar source="interface Scalar {}" type=Scalar
/// @definition.interface symbol=Scalar source="interface Scalar {}"

extension Doubling<T: Scalar> of T {
/// @generic.template symbol=Doubling parameters=(T: Scalar)
/// @definition.extension symbol=Doubling form=local target=T
/// @definition.method symbol=Doubling.double slot=double type=(this: this) => T
/// @type.symbol symbol=Doubling.T source="T: Scalar" type=T
/// @resolution.name source=Scalar target=Scalar
/// @resolution.name source=T target=Doubling.T

    double(this): T {
    /// @type.symbol symbol=Doubling.double type=(this: this) => T
    /// @type.symbol symbol=Doubling.double.this source=this type=this
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
        "",
    );
}
