use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_extension_method_resolution() {
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
const value = point.sum();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
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

const value = point.sum();
/// @resolution.name source=point target=point
/// @resolution.member source=point.sum receiver=Point kind=symbol target=PointMath.sum
/// @resolution.call source="point.sum()" parameters=[] return=int32 kind=symbol target=PointMath.sum receiver=Point
/// @type.symbol symbol=value type=int32
"#);
}

#[test]
fn test_check_reports_missing_unimported_extension_methods() {
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
        DirRows::checked(),
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
fn test_check_records_constrained_generic_extension_resolution() {
    let session = TestSession::single(
        r#"
interface Readable {
    read(): string;
}

struct Box<T> {
    value: T;
}

struct Document {
    read(): string {
        return "ok";
    }
}

extension BoxReadable<T> of Box<T> where T: Readable {
    read(): string {
        return this.value.read();
    }
}

declare const boxed: Box<Document>;
const text = boxed.read();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
interface Readable {
/// @type.symbol symbol=Readable type=Readable

    read(): string;
    /// @type.symbol symbol=Readable.read type=(this: Readable) => string
}

struct Box<T> {
/// @generic.slot symbol=Box.T index=0 kind=type
/// @type.symbol symbol=Box type=Box<T>

    value: T;
    /// @type.symbol symbol=Box.value type=T
}

struct Document {
/// @type.symbol symbol=Document type=Document

    read(): string {
    /// @type.symbol symbol=Document.read type=(this: Document) => string

        return "ok";
    }
}

extension BoxReadable<T> of Box<T> where T: Readable {
/// @generic.slot symbol=BoxReadable.T index=0 kind=type constraint=Readable
/// @resolution.name source=Box target=Box
/// @extension.entry symbol=BoxReadable form=inherent target=Box<BoxReadable.T>

    read(): string {
    /// @type.symbol symbol=BoxReadable.read type=(this: Box<BoxReadable.T>) => string

        return this.value.read();
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Box<BoxReadable.T> kind=symbol target=Box.value
        /// @resolution.member source=this.value.read receiver=BoxReadable.T kind=symbol target=Readable.read
        /// @resolution.call source="this.value.read()" parameters=[] return=string kind=symbol target=Readable.read receiver=BoxReadable.T
    }
}

declare const boxed: Box<Document>;
/// @type.symbol symbol=boxed type=Box<Document>
/// @instance.application source="Box<Document>" id=Box<Document>

const text = boxed.read();
/// @resolution.name source=boxed target=boxed
/// @resolution.member source=boxed.read receiver=Box<Document> kind=symbol target=BoxReadable.read instance=BoxReadable<Document>
/// @resolution.call source="boxed.read()" parameters=[] return=string kind=symbol target=BoxReadable.read receiver=Box<Document> instance=BoxReadable<Document>
/// @type.symbol symbol=text type=string

/// @instance.entry id=Box<Document> symbol=Box arguments=[Document]
/// @instance.entry id=BoxReadable<Document> symbol=BoxReadable arguments=[Document]
"#,
    );
}

#[test]
fn test_check_rejects_extension_when_where_clause_is_not_satisfied() {
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
/// @generic.slot symbol=Box.T index=0 kind=type
/// @type.symbol symbol=Box type=Box<T>

    value: T;
    /// @type.symbol symbol=Box.value type=T
}

struct Token {}
/// @type.symbol symbol=Token type=Token

extension BoxReadable<T> of Box<T> where T: Readable {
/// @generic.slot symbol=BoxReadable.T index=0 kind=type constraint=Readable
/// @resolution.name source=Box target=Box
/// @extension.entry symbol=BoxReadable form=inherent target=Box<BoxReadable.T>

    read(): string {
    /// @type.symbol symbol=BoxReadable.read type=(this: Box<BoxReadable.T>) => string

        return this.value.read();
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Box<BoxReadable.T> kind=symbol target=Box.value
        /// @resolution.member source=this.value.read receiver=BoxReadable.T kind=symbol target=Readable.read
        /// @resolution.call source="this.value.read()" parameters=[] return=string kind=symbol target=Readable.read receiver=BoxReadable.T
    }
}

declare const boxed: Box<Token>;
/// @type.symbol symbol=boxed type=Box<Token>
/// @instance.application source="Box<Token>" id=Box<Token>

boxed.read();
/// @resolution.name source=boxed target=boxed

/// @instance.entry id=Box<Token> symbol=Box arguments=[Token]
"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'read'"
/// @diagnostic.label line=19 column=7 source="boxed.read();"
"#,
    );
}
