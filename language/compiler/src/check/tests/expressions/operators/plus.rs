use crate::tests::{DirRows, TestSession};

#[test]
fn test_builtin_plus_selects_numeric_operator() {
    let session = TestSession::single(
        r#"
const value = 1 + 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: float64 = 1 + 2;

=== checked ===
const value = 1 + 2;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source="1 + 2" type=float64
/// @type.node source=1 type=1
/// @resolution.call source="1 + 2" parameters=() return=float64 kind=builtin builtin=binary.add
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_overloaded_plus_selects_extension_method() {
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
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
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

const sum: Vector = left + right;

=== checked ===
struct Vector {
/// @type.symbol symbol=Vector type=Vector
/// @definition.field symbol=Vector.x source="x: int32" key=x type=int32
/// @definition.field symbol=Vector.y source="y: int32" key=y type=int32
/// @definition.struct symbol=Vector

    x: int32;
    /// @type.symbol symbol=Vector.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Vector.y source="y: int32" type=int32

}

extension VectorAdd of Vector implements Add<Vector> {
/// @relation.entry symbol=VectorAdd kind=implements type=Add<Vector>
/// @definition.extension symbol=VectorAdd form=inherent target=Vector
/// @definition.implements symbol=VectorAdd source=Add<Vector> target=ops.plus.Add arguments=Vector
/// @definition.associated.type symbol=VectorAdd.Output source="type Output = Vector" key=Output value=Vector
/// @definition.method symbol=VectorAdd.add slot=add type=(this: Vector, Vector) => Vector
/// @resolution.name source=Vector target=Vector
/// @resolution.name source=Add target=ops.plus.Add
/// @resolution.name source=Vector target=Vector

    type Output = Vector;
    /// @type.symbol symbol=VectorAdd.Output source="type Output = Vector" type=Vector
    /// @resolution.name source=Vector target=Vector

    add(other: Vector): Vector {
    /// @type.symbol symbol=VectorAdd.add type=(this: Vector, Vector) => Vector
    /// @type.symbol symbol=other source="other: Vector" type=Vector
    /// @resolution.name source=Vector target=Vector
    /// @resolution.name source=Vector target=Vector

        return Vector {
        /// @type.node type=Vector
        /// @resolution.name source=Vector target=Vector

            x: this.x + other.x,
            /// @type.node source="this.x + other.x" type=int32
            /// @type.node source=this type=Vector
            /// @type.node source=this.x type=int32
            /// @resolution.member source=this.x receiver=Vector kind=symbol target=Vector.x
            /// @resolution.call source="this.x + other.x" parameters=() return=int32 kind=builtin builtin=binary.add
            /// @resolution.receiver source=this kind=this owner=VectorAdd type=Vector
            /// @type.node source=other type=Vector
            /// @type.node source=other.x type=int32
            /// @resolution.name source=other target=other
            /// @resolution.member source=other.x receiver=Vector kind=symbol target=Vector.x

            y: this.y + other.y,
            /// @type.node source="this.y + other.y" type=int32
            /// @type.node source=this type=Vector
            /// @type.node source=this.y type=int32
            /// @resolution.member source=this.y receiver=Vector kind=symbol target=Vector.y
            /// @resolution.call source="this.y + other.y" parameters=() return=int32 kind=builtin builtin=binary.add
            /// @resolution.receiver source=this kind=this owner=VectorAdd type=Vector
            /// @type.node source=other type=Vector
            /// @type.node source=other.y type=int32
            /// @resolution.name source=other target=other
            /// @resolution.member source=other.y receiver=Vector kind=symbol target=Vector.y

        };
    }
}

declare const left: Vector;
/// @type.symbol symbol=left source=left type=Vector
/// @resolution.name source=Vector target=Vector

declare const right: Vector;
/// @type.symbol symbol=right source=right type=Vector
/// @resolution.name source=Vector target=Vector

const sum = left + right;
/// @type.symbol symbol=sum source=sum type=Vector
/// @type.node source="left + right" type=Vector
/// @type.node source=left type=Vector
/// @resolution.name source=left target=left
/// @resolution.call source="left + right" parameters=() return=Vector kind=symbol target=VectorAdd.add receiver=Vector
/// @type.node source=right type=Vector
/// @resolution.name source=right target=right
"#);
}
