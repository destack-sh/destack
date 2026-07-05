use crate::tests::{DirRows, TestSession};

#[test]
fn test_struct_member_access_selects_field_symbol() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const point = Point { x: 1 };
const x = point.x;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=int32

const x = point.x;
/// @type.symbol symbol=x source=x type=int32
/// @type.node source=point type=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=symbol target=Point.x

"#,
    );
}

#[test]
fn test_struct_member_call_selects_method_symbol() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;

    length(): int32 {
        return this.x;
    }
}

const point = Point { x: 1 };
const length = point.length();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;

    length(): int32 {
        return this.x;
    }
}

const point: Point = Point { x: 1 };
const length: int32 = point.length();

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.length slot=length type=(this: Point) => int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    length(): int32 {
    /// @type.symbol symbol=Point.length type=(this: Point) => int32

        return this.x;
        /// @type.node source=this type=Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=Point kind=symbol target=Point.x
        /// @resolution.receiver source=this kind=this owner=Point type=Point

    }
}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=int32

const length = point.length();
/// @type.symbol symbol=length source=length type=int32
/// @type.node source=point type=Point
/// @type.node source=point.length type=(this: Point) => int32
/// @type.node source=point.length() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.length receiver=Point kind=symbol target=Point.length
/// @resolution.call source=point.length() parameters=() return=int32 kind=symbol target=Point.length receiver=Point

"#);
}

#[test]
fn test_imported_struct_member_access_selects_exported_field() {
    let compiler = TestSession::builder()
        .module(
            "geometry.ds",
            r#"
export struct Point {
    x: int32;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point } from "./geometry.ds";

const point = Point { x: 1 };
const x = point.x;
"#,
        )
        .build();

    compiler.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Point } from "./geometry.ds";

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== checked ===
import { Point } from "./geometry.ds";

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=geometry.Point
/// @type.node source="Point { x: 1 }" type=geometry.Point
/// @resolution.name source=Point target=geometry.Point
/// @type.node source=1 type=int32

const x = point.x;
/// @type.symbol symbol=x source=x type=int32
/// @type.node source=point type=geometry.Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=geometry.Point kind=symbol target=geometry.Point.x
"#,
    );
}

#[test]
fn test_array_member_access_selects_length() {
    let session = TestSession::single(
        r#"
let values: int32[] = [];
const length = values.length;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
const length: usize = values.length;

=== checked ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @type.node source=[] type=Array<int32>

const length = values.length;
/// @type.symbol symbol=length source=length type=usize
/// @type.node source=values type=Array<int32>
/// @type.node source=values.length type=usize
/// @resolution.name source=values target=values
/// @resolution.member source=values.length receiver=Array<int32> kind=symbol target=collections.array.length#2
"#,
    );
}

#[test]
fn test_array_member_call_selects_push_overload() {
    let session = TestSession::single(
        r#"
let values: int32[] = [];
values.push(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
values.push(1);

=== checked ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @type.node source=[] type=Array<int32>

values.push(1);
/// @type.node source=values type=Array<int32>
/// @type.node source=values.push type=(this: Borrowed<Array<int32>, collections.array.push#1.L0, "exclusive">, int32) => void
/// @type.node source=values.push(1) type=void
/// @resolution.name source=values target=values
/// @resolution.member source=values.push receiver=Array<int32> kind=existential targets=[collections.array.push#1, collections.array.push#2]
/// @resolution.call source=values.push(1) parameters=(int32) return=void kind=symbol target=collections.array.push#1 receiver=Array<int32>
/// @type.node source=1 type=int32

"#,
    );
}

#[test]
fn test_member_on_never_reports_missing_member() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";

function pending(): int32 {
    let value = todo("later");
    return value.field;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";

function pending(): int32 {
    let value: never = todo("later" as string | undefined);
    return value.field;
}

=== checked ===
import { todo } from "destack:error";

function pending(): int32 {
/// @type.symbol symbol=pending type=() => int32

    let value = todo("later");
    /// @type.symbol symbol=pending.value source=value type=never
    /// @resolution.name source=todo target=error.panic.todo
    /// @resolution.call source="todo(\"later\")" parameters=(string | undefined) arguments=(provided("later") as string | undefined) return=never kind=symbol target=error.panic.todo

    return value.field;
    /// @resolution.name source=value target=pending.value

}
"#,
        r#"
/// @diagnostic.error code=EC300 message="member 'field' does not exist on type 'never'"
/// @diagnostic.label line=6 column=18 span="field" line_source="return value.field;"
"#,
    );
}
