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
newtype interface Add<T> {
    type Output;

    add(other: T): this.Output;
}

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
newtype interface Add<T> {
/// @type.symbol symbol=Add type=Add

    type Output;
    /// @type.symbol symbol=Add.Output type=unknown

    add(other: T): this.Output;
    /// @type.symbol symbol=Add.add type=(this: Add, T) => unknown
    /// @type.symbol symbol=other#1 type=T
    /// @resolution.name source=T target=T

}

struct Vector {
/// @type.symbol symbol=Vector type=Vector

    x: int32;
    /// @type.symbol symbol=Vector.x type=int32

    y: int32;
    /// @type.symbol symbol=Vector.y type=int32

}

extension VectorAdd of Vector implements Add<Vector> {
/// @type.symbol symbol=VectorAdd type=VectorAdd
/// @relation.entry symbol=VectorAdd kind=implements type=Add<Vector>
/// @extension.entry symbol=VectorAdd form=inherent target=Vector
/// @resolution.name source=Vector target=Vector
/// @resolution.name source=Add<Vector> target=Add
/// @resolution.name source=Vector target=Vector

    type Output = Vector;
    /// @type.symbol symbol=VectorAdd.Output type=Vector
    /// @resolution.name source=Vector target=Vector

    add(other: Vector): Vector {
    /// @type.symbol symbol=VectorAdd.add type=(this: Vector, Vector) => Vector
    /// @type.symbol symbol=other#2 type=Vector
    /// @resolution.name source=Vector target=Vector
    /// @resolution.name source=Vector target=Vector

        return Vector {
        /// @type.node type=Vector
        /// @resolution.name source=Vector target=Vector

            x: this.x + other.x,
            y: this.y + other.y,
        };
    }
}

declare const left: Vector;
/// @type.symbol symbol=left type=Vector
/// @resolution.name source=Vector target=Vector

declare const right: Vector;
/// @type.symbol symbol=right type=Vector
/// @resolution.name source=Vector target=Vector

const sum = left + right;
/// @type.symbol symbol=sum type=Vector
/// @type.node source="left + right" type=Vector
/// @resolution.name source=left target=left
/// @resolution.member source="left + right" receiver=Vector kind=symbol target=VectorAdd.add
/// @resolution.call source="left + right" parameters=[Vector] return=Vector kind=symbol target=VectorAdd.add receiver=Vector
/// @resolution.name source=right target=right
"#);
}
