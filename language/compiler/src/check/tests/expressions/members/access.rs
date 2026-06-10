use crate::tests::{DirRows, TestSession};

#[test]
fn test_struct_member_access_selects_field_and_method_symbols() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;

    length(): int32 {
        return this.x;
    }
}

const point = Point { x: 1 };
const x = point.x;
const length = point.length();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.struct symbol=Point
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
/// @type.node source=1 type=1

const x = point.x;
/// @type.symbol symbol=x source=x type=int32
/// @type.node source=point type=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=symbol target=Point.x

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
    let compiler = TestSession::new()
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
import { Point } from "./geometry.ds";

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=geometry.Point
/// @type.node source="Point { x: 1 }" type=geometry.Point
/// @resolution.name source=Point target=geometry.Point
/// @type.node source=1 type=1

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
fn test_array_member_access_selects_array_length_and_push() {
    let session = TestSession::single(
        r#"
let values: int32[] = [];
const length = values.length;
values.push(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @type.node source=[] type=Array<int32>

const length = values.length;
/// @type.symbol symbol=length source=length type=number
/// @resolution.name source=values target=values
/// @resolution.member source=values.length receiver=Array<int32> kind=symbol target=collections.array.Array.length
/// @type.node source=values type=Array<int32>
/// @type.node source=values.length type=number

values.push(1);
/// @resolution.name source=values target=values
/// @resolution.member source=values.push receiver=Array<int32> kind=symbol target=collections.array.Array.push
/// @resolution.call source=values.push(1) parameters=(int32) return=void kind=symbol target=collections.array.Array.push receiver=Array<int32>
/// @type.node source=values type=Array<int32>
/// @type.node source=values.push type=(this: Array<int32>, value: int32) => void
/// @type.node source=values.push(1) type=void
/// @type.node source=1 type=int32

"#,
    );
}
