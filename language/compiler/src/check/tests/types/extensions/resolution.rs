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

const value = point.sum();
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=point type=Point
/// @type.node source=point.sum() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.sum receiver=Point kind=symbol target=PointMath.sum
/// @resolution.call source=point.sum() parameters=() return=int32 kind=symbol target=PointMath.sum receiver=Point
"#);
}
