use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_member_and_method_resolution() {
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
        DirRows::checked(),
        r#"
struct Point {
/// @type.symbol symbol=Point type=Point

    x: int32;
    /// @type.symbol symbol=Point.x type=int32

    length(): int32 {
    /// @type.symbol symbol=Point.length type=(this: Point) => int32

        return this.x;
        /// @resolution.member source=this.x receiver=Point kind=symbol target=Point.x
        /// @resolution.receiver source=this kind=this owner=Point type=Point
        /// @type.node source=this type=Point
        /// @type.node source=this.x type=int32

    }
}

const point = Point { x: 1 };
/// @type.symbol symbol=point type=Point
/// @resolution.name source=Point target=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @type.node source=1 type=int32

const x = point.x;
/// @type.symbol symbol=x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=symbol target=Point.x
/// @type.node source=point type=Point
/// @type.node source=point.x type=int32

const length = point.length();
/// @type.symbol symbol=length type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.length receiver=Point kind=symbol target=Point.length
/// @resolution.call source=point.length() parameters=[] return=int32 kind=symbol target=Point.length receiver=Point
/// @type.node source=point type=Point
/// @type.node source=point.length type=(this: Point) => int32
/// @type.node source=point.length() type=int32

"#);
}

#[test]
fn test_check_records_imported_member_targets() {
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
        DirRows::checked(),
        r#"
import { Point } from "./geometry.ds";
/// @type.symbol symbol=Point type=geometry.Point

const point = Point { x: 1 };
/// @type.symbol symbol=point type=geometry.Point
/// @type.node source="Point { x: 1 }" type=geometry.Point
/// @resolution.name source=Point target=geometry.Point
/// @type.node source=1 type=int32

const x = point.x;
/// @type.symbol symbol=x type=int32
/// @type.node source=point type=geometry.Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=geometry.Point kind=symbol target=geometry.Point.x
"#,
    );
}
