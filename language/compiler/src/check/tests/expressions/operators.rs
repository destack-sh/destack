use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_builtin_operator_resolution() {
    let session = TestSession::single(
        r#"
const value = 1 + 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
const value = 1 + 2;
/// @type.symbol symbol=value type=int32
/// @type.node source="1 + 2" type=int32
/// @type.node source=1 type=int32
/// @resolution.call source="1 + 2" parameters=[int32, int32] return=int32 kind=builtin builtin=binary.add
/// @type.node source=2 type=int32
"#,
    );
}

#[test]
fn test_check_records_overloaded_operator_resolution() {
    let session = TestSession::single(
        r#"
struct Vector {
    x: int32;
    y: int32;
}

extension VectorAdd of Vector implements Add<Vector> {
    type Output = Vector;

    add(other: Vector): Vector {
        return Vector {
            x: this.x + other.x,
            y: this.y + other.y,
        };
    }
}

declare const left: Vector;
declare const right: Vector;

const sum = left + right;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
struct Vector {
/// @type.symbol symbol=Vector type=Vector

    x: int32;
    /// @type.symbol symbol=Vector.x type=int32

    y: int32;
    /// @type.symbol symbol=Vector.y type=int32

}

extension VectorAdd of Vector implements Add<Vector> {
/// @relation.entry symbol=VectorAdd kind=implements type=ops.plus.Add<Vector>
/// @extension.entry symbol=VectorAdd form=inherent target=Vector
/// @instance.application source=Add<Vector> id=ops.plus.Add<Vector>

    type Output = Vector;
    /// @type.symbol symbol=VectorAdd.Output type=Vector

    add(other: Vector): Vector {
    /// @type.symbol symbol=VectorAdd.add type=(this: Vector, Vector) => Vector
    /// @type.symbol symbol=other type=Vector

        return Vector {
        /// @type.node type=Vector
        /// @resolution.name source=Vector target=Vector

            x: this.x + other.x,
            /// @type.node source="this.x + other.x" type=int32
            /// @type.node source=this type=Vector
            /// @type.node source=this.x type=int32
            /// @resolution.member source=this.x receiver=Vector kind=symbol target=Vector.x
            /// @resolution.call source="this.x + other.x" parameters=[int32, int32] return=int32 kind=builtin builtin=binary.add
            /// @type.node source=other type=Vector
            /// @type.node source=other.x type=int32
            /// @resolution.name source=other target=other
            /// @resolution.member source=other.x receiver=Vector kind=symbol target=Vector.x

            y: this.y + other.y,
            /// @type.node source="this.y + other.y" type=int32
            /// @type.node source=this type=Vector
            /// @type.node source=this.y type=int32
            /// @resolution.member source=this.y receiver=Vector kind=symbol target=Vector.y
            /// @resolution.call source="this.y + other.y" parameters=[int32, int32] return=int32 kind=builtin builtin=binary.add
            /// @type.node source=other type=Vector
            /// @type.node source=other.y type=int32
            /// @resolution.name source=other target=other
            /// @resolution.member source=other.y receiver=Vector kind=symbol target=Vector.y

        };
    }
}

declare const left: Vector;
/// @type.symbol symbol=left type=Vector

declare const right: Vector;
/// @type.symbol symbol=right type=Vector

const sum = left + right;
/// @type.symbol symbol=sum type=Vector
/// @type.node source="left + right" type=Vector
/// @type.node source=left type=Vector
/// @resolution.name source=left target=left
/// @resolution.member source="left + right" receiver=Vector kind=symbol target=VectorAdd.add
/// @resolution.call source="left + right" parameters=[Vector] return=Vector kind=symbol target=VectorAdd.add receiver=Vector
/// @type.node source=right type=Vector
/// @resolution.name source=right target=right
/// @instance.entry id=ops.plus.Add<Vector> symbol=ops.plus.Add arguments=[Vector]
"#);
}
