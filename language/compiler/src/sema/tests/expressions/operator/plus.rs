use crate::tests::{DirRows, TestSession};

#[test]
fn test_builtin_plus_folds_literal_operands() {
    let session = TestSession::single(
        r#"
const value = 1 + 2;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 3 = 1 + 2;

=== dir ===
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

    add(&readonly this, other: Vector): Vector {
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

    session.assert_dir(
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

    add(&readonly this, other: Vector): Vector {
        return Vector {
            x: this.x + other.x,
            y: this.y + other.y,
        };
    }
}

declare const left: Vector;
declare const right: Vector;

const sum: Vector = left + right;

=== dir ===
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
/// @generic.instance id=Add<Vector> template=Add arguments=(Vector)
/// @definition.extension symbol=<module>#2 form=local target=Vector
/// @definition.implements symbol=<module>#2 source=Add<Vector> target=Add<Vector>
/// @definition.associated.type symbol=Output source="type Output = Vector" key=Output value=Vector
/// @definition.method symbol=add slot=add type=<add.'a>(this: &add.'a readonly Vector, Vector) => Vector
/// @definition.conformance symbol=<module>#2 member=Output requirement=Add.Output
/// @definition.conformance symbol=<module>#2 member=add requirement=Add.add
/// @resolution.name source=Vector target=Vector
/// @resolution.name source=Add target=Add
/// @resolution.name source=Vector target=Vector

    type Output = Vector;
    /// @type.symbol symbol=Output source="type Output = Vector" type=Vector
    /// @resolution.name source=Vector target=Vector

    add(&readonly this, other: Vector): Vector {
    /// @generic.template symbol=add parent=template#0 parameters=('a)
    /// @type.symbol symbol=add type=<add.'a>(this: &add.'a readonly Vector, Vector) => Vector
    /// @type.symbol symbol=add.this source="&readonly this" type=&add.'a readonly Vector
    /// @type.symbol symbol=add.other source="other: Vector" type=Vector
    /// @resolution.name source=Vector target=Vector
    /// @resolution.name source=Vector target=Vector

        return Vector {
        /// @type.node type=Vector
        /// @resolution.name source=Vector target=Vector

            x: this.x + other.x,
            /// @type.node source="this.x + other.x" type=int32
            /// @type.node source=this type=&add.'a readonly Vector
            /// @type.node source=this.x type=int32
            /// @resolution.member source=this.x receiver=&add.'a readonly Vector type=int32 kind=field target_receiver=&add.'a readonly Vector key=x target=Vector.x target_type=int32
            /// @resolution.operator source="this.x + other.x" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), other.x as int32 families=(integer)]
            /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&add.'a readonly Vector
            /// @resolution.place source=this placement=add.'a lifetime=add.'a access="readonly"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.x placement=add.'a lifetime=add.'a access="readonly"
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
            /// @type.node source=this type=&add.'a readonly Vector
            /// @type.node source=this.y type=int32
            /// @resolution.member source=this.y receiver=&add.'a readonly Vector type=int32 kind=field target_receiver=&add.'a readonly Vector key=y target=Vector.y target_type=int32
            /// @resolution.operator source="this.y + other.y" type=int32 operator="+" kind=builtin operands=[this.y as int32 families=(integer), other.y as int32 families=(integer)]
            /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&add.'a readonly Vector
            /// @resolution.place source=this placement=add.'a lifetime=add.'a access="readonly"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.y placement=add.'a lifetime=add.'a access="readonly"
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
/// @resolution.operator source="left + right" type=Vector operator="+" kind=call parameters=(Vector) arguments=(provided(right) as Vector) return=Vector regions=("static" & "local") kind=symbol target=add receiver=Vector adjustments=(borrow(&'static readonly Vector)) instance="Vector.<extension#1>.add<\"static\" & \"local\">"
/// @resolution.place source=left placement="local" lifetime="static" access="immutable"
/// @resolution.access source=left root=left
/// @generic.instantiation id="add<\"static\" & \"local\">" template=add arguments=("static" & "local")
/// @generic.instance id="add<\"bound0\" & \"local\">" template=add arguments=("bound0" & "local")
/// @type.node source=right type=Vector
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="immutable"
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

    add(&readonly this, other: Score): string {
        return "invalid";
    }

    add(&readonly this, other: Score): Score {
        return other;
    }
}

declare const left: Score;
declare const right: Score;

const sum = left + right;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Score {}

extension of Score implements Add<Score> {
    type Output = Score;

    add(&readonly this, other: Score): string {
        return "invalid";
    }

    add(&readonly this, other: Score): Score {
        return other;
    }
}

declare const left: Score;
declare const right: Score;

const sum: Score = left + right;

=== dir ===
struct Score {}
/// @type.symbol symbol=Score source="struct Score {}" type=Score
/// @definition.struct symbol=Score source="struct Score {}"

extension of Score implements Add<Score> {
/// @definition.extension symbol=<module>#2 form=local target=Score
/// @definition.implements symbol=<module>#2 source=Add<Score> target=Add<Score>
/// @definition.associated.type symbol=Output source="type Output = Score" key=Output value=Score
/// @definition.method symbol=add#1 slot=add type=<add#1.'a>(this: &add#1.'a readonly Score, Score) => string
/// @definition.method symbol=add#2 slot=add type=<add#2.'a>(this: &add#2.'a readonly Score, Score) => Score
/// @definition.conformance symbol=<module>#2 member=Output requirement=Add.Output
/// @definition.conformance symbol=<module>#2 member=add#2 requirement=Add.add
/// @resolution.name source=Score target=Score
/// @resolution.name source=Add target=Add
/// @resolution.name source=Score target=Score

    type Output = Score;
    /// @type.symbol symbol=Output source="type Output = Score" type=Score
    /// @resolution.name source=Score target=Score

    add(&readonly this, other: Score): string {
    /// @generic.template symbol=add#1 parent=template#0 parameters=('a)
    /// @type.symbol symbol=add#1 type=<add#1.'a>(this: &add#1.'a readonly Score, Score) => string
    /// @type.symbol symbol=add.this#1 source="&readonly this" type=&add#1.'a readonly Score
    /// @type.symbol symbol=add.other#1 source="other: Score" type=Score
    /// @resolution.name source=Score target=Score

        return "invalid";
        /// @type.node source="\"invalid\"" type="invalid"

    }

    add(&readonly this, other: Score): Score {
    /// @generic.template symbol=add#2 parent=template#0 parameters=('a)
    /// @type.symbol symbol=add#2 type=<add#2.'a>(this: &add#2.'a readonly Score, Score) => Score
    /// @type.symbol symbol=add.this#2 source="&readonly this" type=&add#2.'a readonly Score
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
/// @resolution.operator source="left + right" type=Score operator="+" kind=call parameters=(Score) arguments=(provided(right) as Score) return=Score regions=("static" & "local") kind=symbol target=add#2 receiver=Score adjustments=(borrow(&'static readonly Score)) instance="Score.<extension#1>.add#2<\"static\" & \"local\">"
/// @resolution.place source=left placement="local" lifetime="static" access="immutable"
/// @resolution.access source=left root=left
/// @generic.instantiation id="add#2<\"static\" & \"local\">" template=add#2 arguments=("static" & "local")
/// @type.node source=right type=Score
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="immutable"
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

    session.assert_dir("main.ds", DirRows::checked(), r#"
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

=== dir ===
import { Add } from "destack:ops";

struct Score {
/// @type.symbol symbol=Score type=Score
/// @definition.struct symbol=Score
/// @definition.field symbol=Score.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Score.value source="value: float64" type=float64

}

extension of Score implements Add<Score> {
/// @generic.instance id=Add<Score> template=Add arguments=(Score)
/// @definition.extension symbol=<module>#2 form=local target=Score
/// @definition.implements symbol=<module>#2 source=Add<Score> target=Add<Score>
/// @definition.associated.type symbol=Output source="type Output = Score" key=Output value=Score
/// @definition.method symbol=add slot=add type=<add.'a>(this: &add.'a readonly Score, Score) => Score
/// @definition.conformance symbol=<module>#2 member=Output requirement=Add.Output
/// @definition.conformance symbol=<module>#2 member=add requirement=Add.add
/// @resolution.name source=Score target=Score
/// @resolution.name source=Add target=Add
/// @resolution.name source=Score target=Score

    type Output = Score;
    /// @type.symbol symbol=Output source="type Output = Score" type=Score
    /// @resolution.name source=Score target=Score

    add(other: Score): Score {
    /// @generic.template symbol=add parent=template#0 parameters=('a)
    /// @type.symbol symbol=add type=<add.'a>(this: &add.'a readonly Score, Score) => Score
    /// @type.symbol symbol=add.this type=&add.'a readonly Score
    /// @type.symbol symbol=add.other source="other: Score" type=Score
    /// @resolution.name source=Score target=Score
    /// @resolution.name source=Score target=Score

        Score { value: this.value + other.value }
        /// @resolution.name source=Score target=Score
        /// @resolution.member source=this.value receiver=&add.'a readonly Score type=float64 kind=field target_receiver=&add.'a readonly Score key=value target=Score.value target_type=float64
        /// @resolution.operator source="this.value + other.value" type=float64 operator="+" kind=builtin operands=[this.value as float64 families=(float), other.value as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&add.'a readonly Score
        /// @resolution.place source=this placement=add.'a lifetime=add.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=add.'a lifetime=add.'a access="readonly"
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
/// @resolution.name source=total target=total
/// @resolution.operator source="total += bonus" type=Score operator="+" kind=call parameters=(Score) arguments=(provided(bonus) as Score) return=Score regions=("static" & "local") kind=symbol target=add receiver=Score adjustments=(borrow(&'static readonly Score)) instance="Score.<extension#1>.add<\"static\" & \"local\">"
/// @resolution.pattern.assign source=total kind=place
/// @resolution.place source=total placement="local" lifetime="static" access="exclusive"
/// @resolution.assignment source=total read=binding(total) write=binding(total) type=Score
/// @resolution.access source=total root=total
/// @generic.instantiation id="add<\"static\" & \"local\">" template=add arguments=("static" & "local")
/// @generic.instance id="add<\"bound0\" & \"local\">" template=add arguments=("bound0" & "local")
/// @resolution.name source=bonus target=bonus
/// @resolution.place source=bonus placement="local" lifetime="static" access="immutable"
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

    session.assert_dir("main.ds", DirRows::checked(), r#"
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

=== dir ===
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
/// @definition.implements symbol=<module>#2 source=Add target=Add
/// @definition.associated.type symbol=Output source="type Output = Score" key=Output value=Score
/// @definition.method symbol=add slot=add type=<add.'a>(this: &add.'a readonly Score, Score) => Score
/// @definition.conformance symbol=<module>#2 member=Output requirement=Add.Output
/// @definition.conformance symbol=<module>#2 member=add requirement=Add.add
/// @resolution.name source=Score target=Score
/// @resolution.name source=Add target=Add

    type Output = Score;
    /// @type.symbol symbol=Output source="type Output = Score" type=Score
    /// @resolution.name source=Score target=Score

    add(other: Score): Score {
    /// @generic.template symbol=add parent=template#0 parameters=('a)
    /// @type.symbol symbol=add type=<add.'a>(this: &add.'a readonly Score, Score) => Score
    /// @type.symbol symbol=add.this type=&add.'a readonly Score
    /// @type.symbol symbol=add.other source="other: Score" type=Score
    /// @resolution.name source=Score target=Score
    /// @resolution.name source=Score target=Score

        Score { value: this.value + other.value }
        /// @resolution.name source=Score target=Score
        /// @resolution.member source=this.value receiver=&add.'a readonly Score type=float64 kind=field target_receiver=&add.'a readonly Score key=value target=Score.value target_type=float64
        /// @resolution.operator source="this.value + other.value" type=float64 operator="+" kind=builtin operands=[this.value as float64 families=(float), other.value as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&add.'a readonly Score
        /// @resolution.place source=this placement=add.'a lifetime=add.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=add.'a lifetime=add.'a access="readonly"
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
/// @resolution.name source=total target=total
/// @resolution.operator source="total += bonus" type=Score operator="+" kind=call parameters=(Score) arguments=(provided(bonus) as Score) return=Score regions=("static" & "local") kind=symbol target=add receiver=Score adjustments=(borrow(&'static readonly Score)) instance="Score.<extension#1>.add<\"static\" & \"local\">"
/// @resolution.pattern.assign source=total kind=place
/// @resolution.place source=total placement="local" lifetime="static" access="exclusive"
/// @resolution.assignment source=total read=binding(total) write=binding(total) type=Score
/// @resolution.access source=total root=total
/// @generic.instantiation id="add<\"static\" & \"local\">" template=add arguments=("static" & "local")
/// @generic.instance id="add<\"bound0\" & \"local\">" template=add arguments=("bound0" & "local")
/// @resolution.name source=bonus target=bonus
/// @resolution.place source=bonus placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bonus root=bonus
"#);
}

/// A string literal on the left of `+` dispatches through the string protocol.
#[test]
fn test_add_a_string_literal_receiver_to_a_string() {
    let session = TestSession::single(
        r#"
function wrap(text: string): string {
    return "[" + text + "]";
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function wrap(text: string): string {
    return ("[" + text + "]") as string;
}

=== dir ===
function wrap(text: string): string {
/// @type.symbol symbol=wrap type=(string) => string
/// @type.symbol symbol=wrap.text source="text: string" type=string

    return "[" + text + "]";
    /// @resolution.operator source="\"[\" + text + \"]\"" type=^string operator="+" kind=call parameters=(string) arguments=(provided("]") as string) return=^string regions=("frame" & "local") kind=symbol target=add receiver=^string adjustments=(borrow(&'frame readonly string)) instance="string.<extension#2>.add<\"frame\" & \"local\">"
    /// @resolution.operator source="\"[\" + text" type=^string operator="+" kind=call parameters=(string) arguments=(provided(text) as string) return=^string regions=("managed" & "local") kind=symbol target=add receiver="[" adjustments=(borrow(&'managed readonly "[")) instance="string.<extension#2>.add<\"managed\" & \"local\">"
    /// @generic.instantiation id="add<\"frame\" & \"local\">" template=add arguments=("frame" & "local")
    /// @generic.instantiation id="add<\"managed\" & \"local\">" template=add arguments=("managed" & "local")
    /// @resolution.name source=text target=wrap.text
    /// @resolution.place source=text placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=text root=wrap.text

}
"#,
        r#"
"#,
    );
}
