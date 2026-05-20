use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_intrinsic_operator_resolution() {
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
/// @resolution.call source="1 + 2" parameters=[int32, int32] return=int32 kind=intrinsic
/// @type.node source="1 + 2" type=int32
/// @type.symbol symbol=value type=int32
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
/// @resolution.name source=Vector target=Vector
/// @resolution.name source=Add target=destack:ops.Add
/// @extension.entry symbol=VectorAdd form=inherent target=Vector
/// @relation.entry symbol=VectorAdd kind=implements type=destack:ops.Add<Vector>

    type Output = Vector;
    /// @resolution.name source=Vector target=Vector
    /// @type.symbol symbol=VectorAdd.Output type=Vector

    add(other: Vector): Vector {
    /// @type.symbol symbol=VectorAdd.add type=(this: Vector, Vector) => Vector

        return Vector {
            x: this.x + other.x,
            y: this.y + other.y,
        };
    }
}

declare const left: Vector;
/// @type.symbol symbol=left type=Vector

declare const right: Vector;
/// @type.symbol symbol=right type=Vector

const sum = left + right;
/// @resolution.name source=left target=left
/// @resolution.name source=right target=right
/// @resolution.member source="left + right" receiver=Vector kind=direct target=VectorAdd.add
/// @resolution.call source="left + right" parameters=[Vector] return=Vector kind=direct target=VectorAdd.add receiver=Vector
/// @type.symbol symbol=sum type=Vector
"#);
}
