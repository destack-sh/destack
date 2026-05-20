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
    }
}

const point = Point { x: 1 };
/// @type.symbol symbol=point type=Point

const x = point.x;
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=direct target=Point.x
/// @type.symbol symbol=x type=int32

const length = point.length();
/// @resolution.name source=point target=point
/// @resolution.member source=point.length receiver=Point kind=direct target=Point.length
/// @resolution.call source="point.length()" parameters=[] return=int32 kind=direct target=Point.length receiver=Point
/// @type.symbol symbol=length type=int32

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
/// @resolution.name source=Point target=geometry.Point

const point = Point { x: 1 };
/// @resolution.name source=Point target=geometry.Point
/// @type.symbol symbol=point type=geometry.Point

const x = point.x;
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=geometry.Point kind=direct target=geometry.Point.x
/// @type.symbol symbol=x type=int32
"#,
    );
}
