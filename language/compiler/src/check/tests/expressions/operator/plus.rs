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
/// @resolution.operator source="1 + 2" type=3 operator="+" kind=builtin operands=[1 as 1 families=(integer), 2 as 2 families=(integer)]
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
/// @definition.implements symbol=<module>#2 source=Add<Vector> target="Add<Vector><type Output = Vector>"
/// @definition.associated.type symbol=Output source="type Output = Vector" key=Output value=Vector
/// @definition.method symbol=add slot=add type=(this: this, Vector) => Vector
/// @definition.implementation symbol=<module>#2 requirement=ops.plus.Add.Output target=Output
/// @definition.implementation symbol=<module>#2 requirement=ops.plus.Add.add target=add
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
            /// @resolution.member source=this.x receiver=Vector type=int32 kind=field target_receiver=Vector key=x target=Vector.x target_type=int32
            /// @resolution.operator source="this.x + other.x" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), other.x as int32 families=(integer)]
            /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Vector
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.x placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.x root=this keys=[x]
            /// @type.node source=other type=Vector
            /// @type.node source=other.x type=int32
            /// @resolution.name source=other target=add.other
            /// @resolution.member source=other.x receiver=Vector type=int32 kind=field target_receiver=Vector key=x target=Vector.x target_type=int32
            /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=other root=add.other
            /// @resolution.place source=other.x placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=other.x root=add.other keys=[x]

            y: this.y + other.y,
            /// @type.node source="this.y + other.y" type=int32
            /// @type.node source=this type=Vector
            /// @type.node source=this.y type=int32
            /// @resolution.member source=this.y receiver=Vector type=int32 kind=field target_receiver=Vector key=y target=Vector.y target_type=int32
            /// @resolution.operator source="this.y + other.y" type=int32 operator="+" kind=builtin operands=[this.y as int32 families=(integer), other.y as int32 families=(integer)]
            /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Vector
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.y placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.y root=this keys=[y]
            /// @type.node source=other type=Vector
            /// @type.node source=other.y type=int32
            /// @resolution.name source=other target=add.other
            /// @resolution.member source=other.y receiver=Vector type=int32 kind=field target_receiver=Vector key=y target=Vector.y target_type=int32
            /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=other root=add.other
            /// @resolution.place source=other.y placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=other.y root=add.other keys=[y]

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
/// @resolution.operator source="left + right" type=Vector operator="+" kind=call parameters=(Vector) arguments=(provided(right) as Vector) return=Vector kind=symbol target=add receiver=Vector
/// @resolution.place source=left placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=left root=left
/// @type.node source=right type=Vector
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=right root=right
"#,
    );
}

#[test]
fn test_overloaded_plus_selects_protocol_compatible_declaration() {
    let session = TestSession::single(
        r#"
struct Score {}

extension of Score implements Add<Score> {
    type Output = Score;

    add(other: Score): string {
        return "invalid";
    }

    add(other: Score): Score {
        return other;
    }
}

declare const left: Score;
declare const right: Score;

const sum = left + right;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Score {}

extension of Score implements Add<Score> {
    type Output = Score;

    add(other: Score): string {
        return "invalid";
    }

    add(other: Score): Score {
        return other;
    }
}

declare const left: Score;
declare const right: Score;

const sum: Score = left + right;

=== checked ===
struct Score {}
/// @type.symbol symbol=Score source="struct Score {}" type=Score
/// @definition.struct symbol=Score source="struct Score {}"

extension of Score implements Add<Score> {
/// @definition.extension symbol=<module>#2 form=local target=Score
/// @definition.implements symbol=<module>#2 source=Add<Score> target="Add<Score><type Output = Score>"
/// @definition.associated.type symbol=Output source="type Output = Score" key=Output value=Score
/// @definition.method symbol=add#1 slot=add type=(this: this, Score) => string
/// @definition.method symbol=add#2 slot=add type=(this: this, Score) => Score
/// @definition.implementation symbol=<module>#2 requirement=ops.plus.Add.Output target=Output
/// @definition.implementation symbol=<module>#2 requirement=ops.plus.Add.add target=add#2
/// @resolution.name source=Score target=Score
/// @resolution.name source=Add target=ops.plus.Add
/// @resolution.name source=Score target=Score

    type Output = Score;
    /// @type.symbol symbol=Output source="type Output = Score" type=Score
    /// @resolution.name source=Score target=Score

    add(other: Score): string {
    /// @type.symbol symbol=add#1 type=(this: this, Score) => string
    /// @type.symbol symbol=add.other#1 source="other: Score" type=Score
    /// @resolution.name source=Score target=Score

        return "invalid";
        /// @type.node source="\"invalid\"" type="invalid"

    }

    add(other: Score): Score {
    /// @type.symbol symbol=add#2 type=(this: this, Score) => Score
    /// @type.symbol symbol=add.other#2 source="other: Score" type=Score
    /// @resolution.name source=Score target=Score
    /// @resolution.name source=Score target=Score

        return other;
        /// @type.node source=other type=Score
        /// @resolution.name source=other target=add.other#2
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=add.other#2

    }
}

declare const left: Score;
/// @type.symbol symbol=left source=left type=Score
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=Score target=Score

declare const right: Score;
/// @type.symbol symbol=right source=right type=Score
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=Score target=Score

const sum = left + right;
/// @type.symbol symbol=sum source=sum type=Score
/// @resolution.pattern source=sum kind=binding target=sum
/// @type.node source="left + right" type=Score
/// @type.node source=left type=Score
/// @resolution.name source=left target=left
/// @resolution.operator source="left + right" type=Score operator="+" kind=call parameters=(Score) arguments=(provided(right) as Score) return=Score kind=symbol target=add#2 receiver=Score
/// @resolution.place source=left placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=left root=left
/// @type.node source=right type=Score
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=right root=right
"#,
        r#"
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
/// @definition.implements symbol=<module>#2 source=Add<Score> target="Add<Score><type Output = Score>"
/// @definition.associated.type symbol=Output source="type Output = Score" key=Output value=Score
/// @definition.method symbol=add slot=add type=(this: this, Score) => Score
/// @definition.implementation symbol=<module>#2 requirement=ops.plus.Add.Output target=Output
/// @definition.implementation symbol=<module>#2 requirement=ops.plus.Add.add target=add
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
        /// @resolution.member source=this.value receiver=Score type=float64 kind=field target_receiver=Score key=value target=Score.value target_type=float64
        /// @resolution.operator source="this.value + other.value" type=float64 operator="+" kind=builtin operands=[this.value as float64 families=(float), other.value as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Score
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.name source=other target=add.other
        /// @resolution.member source=other.value receiver=Score type=float64 kind=field target_receiver=Score key=value target=Score.value target_type=float64
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=add.other
        /// @resolution.place source=other.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other.value root=add.other keys=[value]

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
/// @resolution.operator source="total += bonus" type=Score operator="+" kind=call parameters=(Score) arguments=(provided(bonus) as Score) return=Score kind=symbol target=add receiver=Score
/// @resolution.pattern.assign source=total kind=place
/// @resolution.place source=total placement="local" lifetime="static" access="exclusive"
/// @resolution.assignment source=total read=binding(total) write=binding(total) type=Score
/// @resolution.name source=bonus target=bonus
/// @resolution.place source=bonus placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bonus root=bonus
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
/// @definition.implements symbol=<module>#2 source=Add target="Add<this><type Output = Score>"
/// @definition.associated.type symbol=Output source="type Output = Score" key=Output value=Score
/// @definition.method symbol=add slot=add type=(this: this, Score) => Score
/// @definition.implementation symbol=<module>#2 requirement=ops.plus.Add.Output target=Output
/// @definition.implementation symbol=<module>#2 requirement=ops.plus.Add.add target=add
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
        /// @resolution.member source=this.value receiver=Score type=float64 kind=field target_receiver=Score key=value target=Score.value target_type=float64
        /// @resolution.operator source="this.value + other.value" type=float64 operator="+" kind=builtin operands=[this.value as float64 families=(float), other.value as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Score
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.name source=other target=add.other
        /// @resolution.member source=other.value receiver=Score type=float64 kind=field target_receiver=Score key=value target=Score.value target_type=float64
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=add.other
        /// @resolution.place source=other.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other.value root=add.other keys=[value]

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
/// @resolution.operator source="total += bonus" type=Score operator="+" kind=call parameters=(Score) arguments=(provided(bonus) as Score) return=Score kind=symbol target=add receiver=Score
/// @resolution.pattern.assign source=total kind=place
/// @resolution.place source=total placement="local" lifetime="static" access="exclusive"
/// @resolution.assignment source=total read=binding(total) write=binding(total) type=Score
/// @resolution.name source=bonus target=bonus
/// @resolution.place source=bonus placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bonus root=bonus
"#);
}
