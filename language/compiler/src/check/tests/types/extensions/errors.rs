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

    x: int32;
    /// @type.symbol symbol=Point.x type=int32

    y: int32;
    /// @type.symbol symbol=Point.y type=int32
}

extension PointMath of Point {
/// @resolution.name source=Point target=Point
/// @extension.entry symbol=PointMath form=inherent target=Point

    sum(): int32 {
    /// @type.symbol symbol=PointMath.sum type=(this: Point) => int32

        return this.x + this.y;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.x receiver=Point kind=symbol target=Point.x
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.y receiver=Point kind=symbol target=Point.y
        /// @type.node source="this.x + this.y" type=int32
    }
}

declare const point: Point;
/// @type.symbol symbol=point type=Point

point.length();
/// @resolution.name source=point target=point

"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'length'"
/// @diagnostic.label line=14 column=7 source="point.length();"
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
        DirRows::checked(),
        r#"
interface Readable {
/// @type.symbol symbol=Readable type=Readable

    read(): string;
    /// @type.symbol symbol=Readable.read type=(this: Readable) => string
}

struct Box<T> {
/// @generic.template symbol=Box parameters=[T]
/// @type.symbol symbol=Box type=Box<T>

    value: T;
    /// @type.symbol symbol=Box.value type=T
}

struct Token {}
/// @type.symbol symbol=Token type=Token

extension BoxReadable<T> of Box<T> where T: Readable {
/// @generic.template symbol=BoxReadable parameters=[T: Readable]
/// @resolution.name source=Box target=Box
/// @extension.entry symbol=BoxReadable form=inherent target=Box<BoxReadable.T>

    read(): string {
    /// @type.symbol symbol=BoxReadable.read type=(this: Box<BoxReadable.T>) => string

        return this.value.read();
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Box<BoxReadable.T> kind=symbol target=Box.value
        /// @resolution.member source=this.value.read receiver=BoxReadable.T kind=symbol target=Readable.read
        /// @resolution.call source="this.value.read()" parameters=() return=string kind=symbol target=Readable.read receiver=BoxReadable.T
    }
}

declare const boxed: Box<Token>;
/// @type.symbol symbol=boxed type=Box<Token>
/// @generic.application source="Box<Token>" id=Box<Token>

boxed.read();
/// @resolution.name source=boxed target=boxed

/// @generic.application id=Box<Token> symbol=Box arguments=[Token]
"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'read'"
/// @diagnostic.label line=19 column=7 source="boxed.read();"
"#,
    );
}
