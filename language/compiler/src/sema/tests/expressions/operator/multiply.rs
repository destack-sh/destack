use crate::tests::{DirRows, TestSession};

#[test]
fn test_scalar_multiply_projects_output() {
    let session = TestSession::single(
        r#"
import { Multiply } from "tspp:ops";

struct Force {
    value: float64;
}

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(&readonly this, other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const force: Force;
const scaled = force * 2.0;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Multiply } from "tspp:ops";

struct Force {
    value: float64;
}

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(&readonly this, other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const force: Force;
const scaled: Force = force * 2.0;

=== dir ===
import { Multiply } from "tspp:ops";

struct Force {
/// @type.symbol symbol=Force type=Force
/// @definition.struct symbol=Force
/// @definition.field symbol=Force.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Force.value source="value: float64" type=float64

}

extension of Force implements Multiply<float64> {
/// @generic.instance id=Multiply<float64> template=Multiply arguments=(float64)
/// @definition.extension symbol=<module>#2 form=local target=Force
/// @definition.implements symbol=<module>#2 source=Multiply<float64> target=Multiply<float64>
/// @definition.associated.type symbol=Output source="type Output = Force" key=Output value=Force
/// @definition.method symbol=multiply slot=multiply type=<multiply.'a>(this: &multiply.'a readonly Force, float64) => Force
/// @definition.conformance symbol=<module>#2 member=Output requirement=Multiply.Output
/// @definition.conformance symbol=<module>#2 member=multiply requirement=Multiply.multiply
/// @resolution.name source=Force target=Force
/// @resolution.name source=Multiply target=Multiply

    type Output = Force;
    /// @type.symbol symbol=Output source="type Output = Force" type=Force
    /// @resolution.name source=Force target=Force

    multiply(&readonly this, other: float64): Force {
    /// @generic.template symbol=multiply parent=template#0 parameters=('a)
    /// @type.symbol symbol=multiply type=<multiply.'a>(this: &multiply.'a readonly Force, float64) => Force
    /// @type.symbol symbol=multiply.this source="&readonly this" type=&multiply.'a readonly Force
    /// @type.symbol symbol=multiply.other source="other: float64" type=float64
    /// @resolution.name source=Force target=Force

        Force { value: this.value * other }
        /// @resolution.name source=Force target=Force
        /// @resolution.member source=this.value receiver=&multiply.'a readonly Force type=float64 kind=field target_receiver=&multiply.'a readonly Force key=value target=Force.value target_type=float64
        /// @resolution.operator source="this.value * other" type=float64 operator="*" kind=builtin operands=[this.value as float64 families=(float), other as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&multiply.'a readonly Force
        /// @resolution.place source=this placement=multiply.'a lifetime=multiply.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=multiply.'a lifetime=multiply.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.name source=other target=multiply.other
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=multiply.other

    }
}

declare const force: Force;
/// @type.symbol symbol=force source=force type=Force
/// @resolution.pattern source=force kind=binding target=force
/// @resolution.name source=Force target=Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=Force
/// @resolution.pattern source=scaled kind=binding target=scaled
/// @resolution.name source=force target=force
/// @resolution.operator source="force * 2.0" type=Force operator="*" kind=call parameters=(float64) arguments=(provided(2.0) as float64) return=Force regions=("static" & "local") kind=symbol target=multiply receiver=Force adjustments=(borrow(&'static readonly Force)) instance="Force.<extension#1>.multiply<\"static\" & \"local\">"
/// @resolution.place source=force placement="local" lifetime="static" access="immutable"
/// @resolution.access source=force root=force
/// @generic.instantiation id="multiply<\"static\" & \"local\">" template=multiply arguments=("static" & "local")
/// @generic.instance id="multiply<\"bound0\" & \"local\">" template=multiply arguments=("bound0" & "local")
"#,
    );
}

#[test]
fn test_newtype_selects_its_own_multiply() {
    let session = TestSession::single(
        r#"
import { Multiply } from "tspp:ops";

newtype Meters = float64;

extension of Meters implements Multiply<Meters> {
    type Output = float64;

    multiply(&readonly this, other: Meters): float64 {
        todo("Meters.multiply")
    }
}

declare const width: Meters;
declare const height: Meters;
const area = width * height;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Multiply } from "tspp:ops";

newtype Meters = float64;

extension of Meters implements Multiply<Meters> {
    type Output = float64;

    multiply(&readonly this, other: Meters): float64 {
        todo("Meters.multiply" as string | undefined)
    }
}

declare const width: Meters;
declare const height: Meters;
const area: float64 = width * height;

=== dir ===
import { Multiply } from "tspp:ops";

newtype Meters = float64;
/// @type.symbol symbol=Meters source="newtype Meters = float64" type=Meters
/// @definition.newtype symbol=Meters source="newtype Meters = float64" backing=float64 constructors=[(float64) => Meters]

extension of Meters implements Multiply<Meters> {
/// @generic.instance id=Multiply<Meters> template=Multiply arguments=(Meters)
/// @definition.extension symbol=<module>#2 form=local target=Meters
/// @definition.implements symbol=<module>#2 source=Multiply<Meters> target=Multiply<Meters>
/// @definition.associated.type symbol=Output source="type Output = float64" key=Output value=float64
/// @definition.method symbol=multiply slot=multiply type=<multiply.'a>(this: &multiply.'a readonly Meters, Meters) => float64
/// @definition.conformance symbol=<module>#2 member=Output requirement=Multiply.Output
/// @definition.conformance symbol=<module>#2 member=multiply requirement=Multiply.multiply
/// @resolution.name source=Meters target=Meters
/// @resolution.name source=Multiply target=Multiply
/// @resolution.name source=Meters target=Meters

    type Output = float64;
    /// @type.symbol symbol=Output source="type Output = float64" type=float64

    multiply(&readonly this, other: Meters): float64 {
    /// @generic.template symbol=multiply parent=template#0 parameters=('a)
    /// @type.symbol symbol=multiply type=<multiply.'a>(this: &multiply.'a readonly Meters, Meters) => float64
    /// @type.symbol symbol=multiply.this source="&readonly this" type=&multiply.'a readonly Meters
    /// @type.symbol symbol=multiply.other source="other: Meters" type=Meters
    /// @resolution.name source=Meters target=Meters

        todo("Meters.multiply")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Meters.multiply\")" parameters=(string | undefined) arguments=(provided("Meters.multiply") as string | undefined) return=never kind=symbol target=todo

    }
}

declare const width: Meters;
/// @type.symbol symbol=width source=width type=Meters
/// @resolution.pattern source=width kind=binding target=width
/// @resolution.name source=Meters target=Meters

declare const height: Meters;
/// @type.symbol symbol=height source=height type=Meters
/// @resolution.pattern source=height kind=binding target=height
/// @resolution.name source=Meters target=Meters

const area = width * height;
/// @type.symbol symbol=area source=area type=float64
/// @resolution.pattern source=area kind=binding target=area
/// @resolution.name source=width target=width
/// @resolution.operator source="width * height" type=float64 operator="*" kind=call parameters=(Meters) arguments=(provided(height) as Meters) return=float64 regions=("static" & "local") kind=symbol target=multiply receiver=Meters adjustments=(borrow(&'static readonly Meters)) instance="Meters.<extension#1>.multiply<\"static\" & \"local\">"
/// @resolution.place source=width placement="local" lifetime="static" access="immutable"
/// @resolution.access source=width root=width
/// @generic.instantiation id="multiply<\"static\" & \"local\">" template=multiply arguments=("static" & "local")
/// @generic.instance id="multiply<\"bound0\" & \"local\">" template=multiply arguments=("bound0" & "local")
/// @resolution.name source=height target=height
/// @resolution.place source=height placement="local" lifetime="static" access="immutable"
/// @resolution.access source=height root=height
"#,
    );
}
