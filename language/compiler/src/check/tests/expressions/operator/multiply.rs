use crate::tests::{DirRows, TestSession};

#[test]
fn test_scalar_multiply_projects_output() {
    let session = TestSession::single(
        r#"
import { Multiply } from "destack:ops";

struct Force {
    value: float64;
}

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const force: Force;
const scaled = force * 2.0;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Multiply } from "destack:ops";

struct Force {
    value: float64;
}

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const force: Force;
const scaled: Force = force * 2.0;

=== checked ===
import { Multiply } from "destack:ops";

struct Force {
/// @type.symbol symbol=Force type=Force
/// @definition.struct symbol=Force
/// @definition.field symbol=Force.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Force.value source="value: float64" type=float64

}

extension of Force implements Multiply<float64> {
/// @definition.extension symbol=<module>#2 form=local target=Force
/// @definition.implements symbol=<module>#2 source=Multiply<float64> target=ops.multiply.Multiply arguments=(float64)
/// @definition.associated.type symbol=Output source="type Output = Force" key=Output value=Force
/// @definition.method symbol=multiply slot=multiply type=(this: Force, float64) => Force
/// @resolution.name source=Force target=Force
/// @resolution.name source=Multiply target=ops.multiply.Multiply

    type Output = Force;
    /// @type.symbol symbol=Output source="type Output = Force" type=Force
    /// @resolution.name source=Force target=Force

    multiply(other: float64): Force {
    /// @type.symbol symbol=multiply type=(this: Force, float64) => Force
    /// @type.symbol symbol=multiply.other source="other: float64" type=float64
    /// @resolution.name source=Force target=Force

        Force { value: this.value * other }
        /// @resolution.name source=Force target=Force
        /// @resolution.member source=this.value receiver=Force kind=symbol target=Force.value
        /// @resolution.call source="this.value * other" parameters=() return=float64 kind=builtin builtin=binary.multiply
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Force
        /// @resolution.name source=other target=multiply.other

    }
}

declare const force: Force;
/// @type.symbol symbol=force source=force type=Force
/// @resolution.name source=Force target=Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=Force
/// @resolution.name source=force target=force
/// @resolution.call source="force * 2.0" parameters=(float64) arguments=(provided(2.0) as float64) return=Force kind=symbol target=multiply receiver=Force
"#,
    );
}

#[test]
fn test_newtype_selects_its_own_multiply() {
    let session = TestSession::single(
        r#"
import { Multiply } from "destack:ops";

newtype Meters = float64;

extension of Meters implements Multiply<Meters> {
    type Output = float64;

    multiply(other: Meters): float64 {
        todo("Meters.multiply")
    }
}

declare const width: Meters;
declare const height: Meters;
const area = width * height;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Multiply } from "destack:ops";

newtype Meters = float64;

extension of Meters implements Multiply<Meters> {
    type Output = float64;

    multiply(other: Meters): float64 {
        todo("Meters.multiply" as string | undefined)
    }
}

declare const width: Meters;
declare const height: Meters;
const area: float64 = width * height;

=== checked ===
import { Multiply } from "destack:ops";

newtype Meters = float64;
/// @type.symbol symbol=Meters source="newtype Meters = float64" type=Meters
/// @definition.newtype symbol=Meters source="newtype Meters = float64" value=float64

extension of Meters implements Multiply<Meters> {
/// @definition.extension symbol=<module>#2 form=local target=Meters
/// @definition.implements symbol=<module>#2 source=Multiply<Meters> target=ops.multiply.Multiply arguments=(Meters)
/// @definition.associated.type symbol=Output source="type Output = float64" key=Output value=float64
/// @definition.method symbol=multiply slot=multiply type=(this: Meters, Meters) => float64
/// @resolution.name source=Meters target=Meters
/// @resolution.name source=Multiply target=ops.multiply.Multiply
/// @resolution.name source=Meters target=Meters

    type Output = float64;
    /// @type.symbol symbol=Output source="type Output = float64" type=float64

    multiply(other: Meters): float64 {
    /// @type.symbol symbol=multiply type=(this: Meters, Meters) => float64
    /// @type.symbol symbol=multiply.other source="other: Meters" type=Meters
    /// @resolution.name source=Meters target=Meters

        todo("Meters.multiply")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Meters.multiply\")" parameters=(string | undefined) arguments=(provided("Meters.multiply") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

declare const width: Meters;
/// @type.symbol symbol=width source=width type=Meters
/// @resolution.name source=Meters target=Meters

declare const height: Meters;
/// @type.symbol symbol=height source=height type=Meters
/// @resolution.name source=Meters target=Meters

const area = width * height;
/// @type.symbol symbol=area source=area type=float64
/// @resolution.name source=width target=width
/// @resolution.call source="width * height" parameters=(Meters) arguments=(provided(height) as Meters) return=float64 kind=symbol target=multiply receiver=Meters
/// @resolution.name source=height target=height
"#,
    );
}
