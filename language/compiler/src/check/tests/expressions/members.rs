use super::super::snapshot::{assert_check_module_snapshot, assert_check_snapshot};
use crate::tests::TestCompiler;

#[test]
fn test_check_records_member_and_method_resolution() {
    assert_check_snapshot(
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
        r#"
struct Point {
/// @type.symbol key=Point value=Point
/// @layout.type type=Point layout=layout0 shape=struct

    x: int32;
    /// @type.symbol key=Point.x value=int32

    length(): int32 {
    /// @type.symbol key=Point.length value=(this: Point) => int32

        return this.x;
    }
}

const point = Point { x: 1 };
/// @type.symbol key=point value=Point

const x = point.x;
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=direct target=Point.x
/// @type.symbol key=x value=int32

const length = point.length();
/// @resolution.member source=point.length receiver=Point kind=direct target=Point.length
/// @resolution.call source="point.length()" parameters=[] return=int32 kind=direct target=Point.length receiver=Point
/// @type.symbol key=length value=int32

/// @layout.entry layout=layout0 shape=struct
/// @layout.summary layouts=1 types=1
/// @type.summary types=3 nodes=3 symbols=6
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=2 calls=1
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
"#,
    );
}

#[test]
fn test_check_records_imported_member_targets() {
    let compiler = TestCompiler::new()
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

    assert_check_module_snapshot(
        &compiler,
        "main.ds",
        r#"
import { Point } from "./geometry.ds";
/// @resolution.name source=Point target=geometry.Point

const point = Point { x: 1 };
/// @resolution.name source=Point target=geometry.Point
/// @type.symbol key=point value=geometry.Point

const x = point.x;
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=geometry.Point kind=direct target=geometry.Point.x
/// @type.symbol key=x value=int32

/// @type.summary types=2 nodes=0 symbols=2
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=3 labels=0 members=1 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}
