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
    sum(): int32 {
        return this.x + this.y;
    }
}

declare const point: Point;
const value = point.sum();
"#,
    );

    session.assert_dir_checked(
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
const value: int32 = point.sum();

=== checked ===
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
/// @definition.method symbol=sum slot=sum type=(this: this) => int32
/// @resolution.name source=Point target=Point

    sum(): int32 {
    /// @type.symbol symbol=sum type=(this: this) => int32

        return this.x + this.y;
        /// @type.node source="this.x + this.y" type=int32
        /// @type.node source=this type=Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=Point kind=symbol target=Point.x
        /// @resolution.operator source="this.x + this.y" kind=builtin
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Point
        /// @type.node source=this type=Point
        /// @type.node source=this.y type=int32
        /// @resolution.member source=this.y receiver=Point kind=symbol target=Point.y
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Point

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
/// @type.node source=point.sum type=(this: Point) => int32
/// @type.node source=point.sum() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.sum receiver=Point kind=symbol target=sum
/// @resolution.call source=point.sum() parameters=() return=int32 kind=symbol target=sum receiver=Point
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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
/// @definition.method symbol=sum slot=sum type=(this: this) => int32
/// @resolution.name source=Point target=Point

    sum(): int32 {
    /// @type.symbol symbol=sum type=(this: this) => int32

        return this.x + this.y;
        /// @type.node source="this.x + this.y" type=int32
        /// @type.node source=this type=Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=Point kind=symbol target=Point.x
        /// @resolution.operator source="this.x + this.y" kind=builtin
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Point
        /// @type.node source=this type=Point
        /// @type.node source=this.y type=int32
        /// @resolution.member source=this.y receiver=Point kind=symbol target=Point.y
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Point

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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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
/// @definition.method symbol=first slot=first type=<first.'l0>(this: &first.'l0 readonly Slice<T#2>) => usize
/// @definition.method symbol=size slot=size role=getter type=<size.'l0>(this: &size.'l0 readonly Slice<T#2>) => usize
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Slice target=Slice
/// @resolution.name source=T target=T

    get size(this: &readonly Slice<T>): usize {
    /// @generic.template symbol=size parent=template#1 parameters=('l0)
    /// @type.symbol symbol=size type=<size.'l0>(this: &size.'l0 readonly Slice<T#2>) => usize
    /// @type.symbol symbol=size.this source="this: &readonly Slice<T>" type=&size.'l0 readonly Slice<T#2>
    /// @resolution.name source=Slice target=Slice
    /// @resolution.name source=T target=T

        this.length
        /// @resolution.member source=this.length receiver=&size.'l0 readonly Slice<T#2> kind=symbol target=Slice.length
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&size.'l0 readonly Slice<T#2>

    }

    first(this: &readonly Slice<T>): usize {
    /// @generic.template symbol=first parent=template#1 parameters=('l0)
    /// @type.symbol symbol=first type=<first.'l0>(this: &first.'l0 readonly Slice<T#2>) => usize
    /// @type.symbol symbol=first.this source="this: &readonly Slice<T>" type=&first.'l0 readonly Slice<T#2>
    /// @resolution.name source=Slice target=Slice
    /// @resolution.name source=T target=T

        this.size
        /// @resolution.member source=this.size receiver=&first.'l0 readonly Slice<T#2> kind=symbol target=size
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&first.'l0 readonly Slice<T#2>

    }
}

/// @generic.instance id=Slice<T#2> template=Slice arguments=(T#2)
"#,
        r#""#,
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

    session.assert_dir_checked_and_diagnostics(
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
        this.grow();
    }
}

=== checked ===
struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.length source="length: usize" key=length type=usize

    length: usize;
    /// @type.symbol symbol=Buffer.length source="length: usize" type=usize

}

extension of Buffer {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.method symbol=grow source="grow(this: &exclusive Buffer): void {}" slot=grow type=<grow.'l0>(this: &grow.'l0 exclusive Buffer) => void
/// @definition.method symbol=peek slot=peek type=<peek.'l0>(this: &peek.'l0 readonly Buffer) => void
/// @resolution.name source=Buffer target=Buffer

    grow(this: &exclusive Buffer): void {}
    /// @generic.template symbol=grow parameters=('l0)
    /// @type.symbol symbol=grow source="grow(this: &exclusive Buffer): void {}" type=<grow.'l0>(this: &grow.'l0 exclusive Buffer) => void
    /// @type.symbol symbol=grow.this source="this: &exclusive Buffer" type=&grow.'l0 exclusive Buffer
    /// @resolution.name source=Buffer target=Buffer

    peek(this: &readonly Buffer): void {
    /// @generic.template symbol=peek parameters=('l0)
    /// @type.symbol symbol=peek type=<peek.'l0>(this: &peek.'l0 readonly Buffer) => void
    /// @type.symbol symbol=peek.this source="this: &readonly Buffer" type=&peek.'l0 readonly Buffer
    /// @resolution.name source=Buffer target=Buffer

        this.grow()
        /// @resolution.member source=this.grow receiver=&peek.'l0 readonly Buffer kind=symbol target=grow
        /// @resolution.call source=this.grow() parameters=() return=void kind=symbol target=grow receiver=&peek.'l0 readonly Buffer
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&peek.'l0 readonly Buffer

    }
}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'l0 readonly Buffer' is not assignable to the method's 'this' type '&exclusive Buffer'"
/// @diagnostic.label line=10 column=9 span="this.grow()" line_source="this.grow()"
"#,
    );
}
