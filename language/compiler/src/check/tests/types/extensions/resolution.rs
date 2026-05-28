use crate::tests::{DirRows, TestSession};

#[test]
fn test_extension_method_call_selects_imported_extension() {
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

const value = point.sum();
/// @resolution.name source=point target=point
/// @resolution.member source=point.sum receiver=Point kind=symbol target=PointMath.sum
/// @resolution.call source="point.sum()" parameters=() return=int32 kind=symbol target=PointMath.sum receiver=Point
/// @type.symbol symbol=value type=int32
"#);
}
