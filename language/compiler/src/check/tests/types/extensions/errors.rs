use crate::tests::{DirRows, TestSession};

#[test]
fn test_unimported_extension_method_call_reports_error() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

extension PointMath of Point {
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
struct Point {
/// @type.symbol symbol=Point type=Point
/// @nominal.field symbol=Point.x source="x: int32" key=x type=int32
/// @nominal.field symbol=Point.y source="y: int32" key=y type=int32
/// @nominal.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

extension PointMath of Point {
/// @extension.entry symbol=PointMath form=inherent target=Point
/// @resolution.name source=Point target=Point

    sum(): int32 {
    /// @type.symbol symbol=PointMath.sum type=(this: Point) => int32

        return this.x + this.y;
        /// @type.node source="this.x + this.y" type=int32
        /// @type.node source=this type=Point
        /// @type.node source=this.x type=int32
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.x receiver=Point kind=symbol target=Point.x
        /// @resolution.call source="this.x + this.y" parameters=(int32, int32) return=int32 kind=builtin builtin=binary.add
        /// @type.node source=this type=Point
        /// @type.node source=this.y type=int32
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.y receiver=Point kind=symbol target=Point.y

    }
}

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.name source=Point target=Point

point.length();
/// @type.node source=point type=Point
/// @type.node source=point.length() type=<error>
/// @resolution.name source=point target=point

"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'length'"
/// @diagnostic.label line=14 column=1 source="point.length();"
"#,
    );
}

#[test]
fn test_extension_call_without_satisfied_where_clause_reports_error() {
    let session = TestSession::single(
        r#"
interface Readable {
    read(): string;
}

struct Box<T> {
    value: T;
}

struct Token {}

extension BoxReadable<T> of Box<T> where T: Readable {
    read(): string {
        return this.value.read();
    }
}

declare const boxed: Box<Token>;
boxed.read();
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
interface Readable {
/// @type.symbol symbol=Readable type=Readable

    read(): string;
    /// @type.symbol symbol=Readable.read source="read(): string" type=() => string

}

struct Box<T> {
/// @generic.template symbol=Box parameters=[T#1]
/// @type.symbol symbol=Box type=Box<T#1>
/// @nominal.field symbol=Box.value source="value: T" key=value type=T#1
/// @nominal.struct symbol=Box template=LocalGenericTemplateId(0)
/// @type.symbol symbol=T#1 source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=T#1

}

struct Token {}
/// @type.symbol symbol=Token source="struct Token {}" type=Token
/// @nominal.struct symbol=Token source="struct Token {}"

extension BoxReadable<T> of Box<T> where T: Readable {
/// @generic.template symbol=BoxReadable parameters=[T#2: Readable]
/// @extension.entry symbol=BoxReadable form=inherent target=Box<T#2>
/// @type.symbol symbol=T#2 source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#2
/// @resolution.name source=T target=T#2
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @type.symbol symbol=BoxReadable.read type=(this: Box<T#2>) => string

        return this.value.read();
        /// @generic.instance source=this.value id=Box<T#2>
        /// @type.node source=this type=Box<T#2>
        /// @type.node source=this.value type=T#2
        /// @type.node source=this.value.read() type=string
        /// @resolution.name source=this target=this#2
        /// @resolution.member source=this.value receiver=Box<T#2> kind=symbol target=Box.value instance=Box<T#2>
        /// @resolution.member source=this.value.read receiver=T#2 kind=symbol target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string kind=symbol target=Readable.read receiver=T#2

    }
}

declare const boxed: Box<Token>;
/// @type.symbol symbol=boxed source=boxed type=Box<Token>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Token target=Token

boxed.read();
/// @type.node source=boxed type=Box<Token>
/// @type.node source=boxed.read() type=<error>
/// @resolution.name source=boxed target=boxed

/// @generic.instance id=Box<T#2> symbol=Box arguments=[T#2]
"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'read'"
/// @diagnostic.label line=19 column=1 source="boxed.read();"
"#,
    );
}
