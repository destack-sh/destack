use crate::tests::{DirRows, TestSession};

#[test]
fn test_builtin_plus_folds_literal_operands() {
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
const value: 3 = 1 + 2;

=== checked ===
const value = 1 + 2;
/// @type.symbol symbol=value source=value type=3
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="1 + 2" type=3
/// @type.node source=1 type=1
/// @resolution.operator source="1 + 2" kind=builtin
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

extension of Vector implements Add<Vector> {
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

extension of Vector implements Add<Vector> {
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
/// @definition.struct symbol=Vector
/// @definition.field symbol=Vector.x source="x: int32" key=x type=int32
/// @definition.field symbol=Vector.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Vector.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Vector.y source="y: int32" type=int32

}

extension of Vector implements Add<Vector> {
/// @definition.extension symbol=<module>#2 form=local target=Vector
/// @definition.implements symbol=<module>#2 source=Add<Vector> target=ops.plus.Add arguments=(Vector)
/// @definition.associated.type symbol=Output source="type Output = Vector" key=Output value=Vector
/// @definition.method symbol=add slot=add type=(this: this, Vector) => Vector
/// @resolution.name source=Vector target=Vector
/// @resolution.name source=Add target=ops.plus.Add
/// @resolution.name source=Vector target=Vector

    type Output = Vector;
    /// @type.symbol symbol=Output source="type Output = Vector" type=Vector
    /// @resolution.name source=Vector target=Vector

    add(other: Vector): Vector {
    /// @type.symbol symbol=add type=(this: this, Vector) => Vector
    /// @type.symbol symbol=add.other source="other: Vector" type=Vector
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
            /// @resolution.operator source="this.x + other.x" kind=builtin
            /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Vector
            /// @type.node source=other type=Vector
            /// @type.node source=other.x type=int32
            /// @resolution.name source=other target=add.other
            /// @resolution.member source=other.x receiver=Vector kind=symbol target=Vector.x

            y: this.y + other.y,
            /// @type.node source="this.y + other.y" type=int32
            /// @type.node source=this type=Vector
            /// @type.node source=this.y type=int32
            /// @resolution.member source=this.y receiver=Vector kind=symbol target=Vector.y
            /// @resolution.operator source="this.y + other.y" kind=builtin
            /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Vector
            /// @type.node source=other type=Vector
            /// @type.node source=other.y type=int32
            /// @resolution.name source=other target=add.other
            /// @resolution.member source=other.y receiver=Vector kind=symbol target=Vector.y

        };
    }
}

declare const left: Vector;
/// @type.symbol symbol=left source=left type=Vector
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=Vector target=Vector

declare const right: Vector;
/// @type.symbol symbol=right source=right type=Vector
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=Vector target=Vector

const sum = left + right;
/// @type.symbol symbol=sum source=sum type=Vector
/// @resolution.pattern source=sum kind=binding target=sum
/// @type.node source="left + right" type=Vector
/// @type.node source=left type=Vector
/// @resolution.name source=left target=left
/// @resolution.operator source="left + right" kind=call parameters=(Vector) arguments=(provided(right) as Vector) return=Vector target=add receiver=Vector
/// @type.node source=right type=Vector
/// @resolution.name source=right target=right
"#,
    );
}

#[test]
fn test_compound_assignment_selects_extension_method() {
    let session = TestSession::single(
        r#"
import { Add } from "destack:ops";

struct Score {
    value: float64;
}

extension of Score implements Add<Score> {
    type Output = Score;

    add(other: Score): Score {
        Score { value: this.value + other.value }
    }
}

declare let total: Score;
declare const bonus: Score;
total += bonus;
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Add } from "destack:ops";

struct Score {
    value: float64;
}

extension of Score implements Add<Score> {
    type Output = Score;

    add(other: Score): Score {
        Score { value: this.value + other.value }
    }
}

declare let total: Score;
declare const bonus: Score;
total += bonus;

=== checked ===
import { Add } from "destack:ops";

struct Score {
/// @type.symbol symbol=Score type=Score
/// @definition.struct symbol=Score
/// @definition.field symbol=Score.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Score.value source="value: float64" type=float64

}

extension of Score implements Add<Score> {
/// @definition.extension symbol=<module>#2 form=local target=Score
/// @definition.implements symbol=<module>#2 source=Add<Score> target=ops.plus.Add arguments=(Score)
/// @definition.associated.type symbol=Output source="type Output = Score" key=Output value=Score
/// @definition.method symbol=add slot=add type=(this: this, Score) => Score
/// @resolution.name source=Score target=Score
/// @resolution.name source=Add target=ops.plus.Add
/// @resolution.name source=Score target=Score

    type Output = Score;
    /// @type.symbol symbol=Output source="type Output = Score" type=Score
    /// @resolution.name source=Score target=Score

    add(other: Score): Score {
    /// @type.symbol symbol=add type=(this: this, Score) => Score
    /// @type.symbol symbol=add.other source="other: Score" type=Score
    /// @resolution.name source=Score target=Score
    /// @resolution.name source=Score target=Score

        Score { value: this.value + other.value }
        /// @resolution.name source=Score target=Score
        /// @resolution.member source=this.value receiver=Score kind=symbol target=Score.value
        /// @resolution.operator source="this.value + other.value" kind=builtin
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Score
        /// @resolution.name source=other target=add.other
        /// @resolution.member source=other.value receiver=Score kind=symbol target=Score.value

    }
}

declare let total: Score;
/// @type.symbol symbol=total source=total type=Score
/// @resolution.pattern source=total kind=binding target=total
/// @resolution.name source=Score target=Score

declare const bonus: Score;
/// @type.symbol symbol=bonus source=bonus type=Score
/// @resolution.pattern source=bonus kind=binding target=bonus
/// @resolution.name source=Score target=Score

total += bonus;
/// @resolution.operator source="total += bonus" kind=call parameters=(Score) arguments=(provided(bonus) as Score) return=Score target=add receiver=Score
/// @resolution.pattern.assign source=total kind=place place=binding(total) type=Score
/// @resolution.name source=bonus target=bonus
"#);
}

#[test]
fn test_implements_argument_defaults_to_the_implementer() {
    let session = TestSession::single(
        r#"
import { Add } from "destack:ops";

struct Score {
    value: float64;
}

extension of Score implements Add {
    type Output = Score;

    add(other: Score): Score {
        Score { value: this.value + other.value }
    }
}

declare let total: Score;
declare const bonus: Score;
total += bonus;
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Add } from "destack:ops";

struct Score {
    value: float64;
}

extension of Score implements Add {
    type Output = Score;

    add(other: Score): Score {
        Score { value: this.value + other.value }
    }
}

declare let total: Score;
declare const bonus: Score;
total += bonus;

=== checked ===
import { Add } from "destack:ops";

struct Score {
/// @type.symbol symbol=Score type=Score
/// @definition.struct symbol=Score
/// @definition.field symbol=Score.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Score.value source="value: float64" type=float64

}

extension of Score implements Add {
/// @definition.extension symbol=<module>#2 form=local target=Score
/// @definition.implements symbol=<module>#2 source=Add target=ops.plus.Add arguments=(this)
/// @definition.associated.type symbol=Output source="type Output = Score" key=Output value=Score
/// @definition.method symbol=add slot=add type=(this: this, Score) => Score
/// @resolution.name source=Score target=Score
/// @resolution.name source=Add target=ops.plus.Add

    type Output = Score;
    /// @type.symbol symbol=Output source="type Output = Score" type=Score
    /// @resolution.name source=Score target=Score

    add(other: Score): Score {
    /// @type.symbol symbol=add type=(this: this, Score) => Score
    /// @type.symbol symbol=add.other source="other: Score" type=Score
    /// @resolution.name source=Score target=Score
    /// @resolution.name source=Score target=Score

        Score { value: this.value + other.value }
        /// @resolution.name source=Score target=Score
        /// @resolution.member source=this.value receiver=Score kind=symbol target=Score.value
        /// @resolution.operator source="this.value + other.value" kind=builtin
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Score
        /// @resolution.name source=other target=add.other
        /// @resolution.member source=other.value receiver=Score kind=symbol target=Score.value

    }
}

declare let total: Score;
/// @type.symbol symbol=total source=total type=Score
/// @resolution.pattern source=total kind=binding target=total
/// @resolution.name source=Score target=Score

declare const bonus: Score;
/// @type.symbol symbol=bonus source=bonus type=Score
/// @resolution.pattern source=bonus kind=binding target=bonus
/// @resolution.name source=Score target=Score

total += bonus;
/// @resolution.operator source="total += bonus" kind=call parameters=(Score) arguments=(provided(bonus) as Score) return=Score target=add receiver=Score
/// @resolution.pattern.assign source=total kind=place place=binding(total) type=Score
/// @resolution.name source=bonus target=bonus
"#);
}
