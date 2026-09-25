use crate::tests::{DirRows, TestSession};

#[test]
fn test_exported_extension_overloads_imported_type() {
    let session = TestSession::builder()
        .module(
            "force.tspp",
            r#"
import { Multiply } from "tspp:ops";

export struct Force {
    value: float64;
}

export extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(&readonly this, other: float64): Force {
        Force { value: this.value * other }
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Force } from "./force.tspp";

declare const force: Force;
const scaled = force * 2.0;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Force } from "./force.tspp";

declare const force: Force;
const scaled: Force = force * 2.0;

=== dir ===
import { Force } from "./force.tspp";

declare const force: Force;
/// @type.symbol symbol=force source=force type=force.Force
/// @resolution.pattern source=force kind=binding target=force
/// @resolution.name source=Force target=force.Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=force.Force
/// @resolution.pattern source=scaled kind=binding target=scaled
/// @resolution.name source=force target=force
/// @resolution.operator source="force * 2.0" type=force.Force operator="*" kind=call parameters=(float64) arguments=(provided(2.0) as float64) return=force.Force regions=("static" & "local") kind=symbol target=force.multiply receiver=force.Force adjustments=(borrow(&'static readonly force.Force)) instance="force.Force.<extension#1>.multiply<\"static\" & \"local\">"
/// @resolution.place source=force placement="local" lifetime="static" access="immutable"
/// @resolution.access source=force root=force
/// @generic.instantiation id="force.multiply<\"static\" & \"local\">" template=force.multiply arguments=("static" & "local")
/// @generic.instance id="force.multiply<\"bound0\" & \"local\">" template=force.multiply arguments=("bound0" & "local")
"#,
    );
}

#[test]
fn test_local_extension_stays_module_private() {
    let session = TestSession::builder()
        .module(
            "force.tspp",
            r#"
import { Multiply } from "tspp:ops";

export struct Force {
    value: float64;
}

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(&readonly this, other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const inside: Force;
export const doubled: Force = inside * 2.0;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Force } from "./force.tspp";

declare const force: Force;
const scaled = force * 2.0;
"#,
        )
        .build();

    // resolve the operator inside the declaring module
    session.assert_dir(
        "force.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Multiply } from "tspp:ops";

export struct Force {
    value: float64;
}

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(&readonly this, other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const inside: Force;
export const doubled: Force = inside * 2.0;

=== dir ===
import { Multiply } from "tspp:ops";

export struct Force {
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

declare const inside: Force;
/// @type.symbol symbol=inside source=inside type=Force
/// @resolution.pattern source=inside kind=binding target=inside
/// @resolution.name source=Force target=Force

export const doubled: Force = inside * 2.0;
/// @type.symbol symbol=doubled source=doubled type=Force
/// @resolution.pattern source=doubled kind=binding target=doubled
/// @resolution.name source=Force target=Force
/// @resolution.name source=inside target=inside
/// @resolution.operator source="inside * 2.0" type=Force operator="*" kind=call parameters=(float64) arguments=(provided(2.0) as float64) return=Force regions=("static" & "local") kind=symbol target=multiply receiver=Force adjustments=(borrow(&'static readonly Force)) instance="Force.<extension#1>.multiply<\"static\" & \"local\">"
/// @resolution.place source=inside placement="local" lifetime="static" access="immutable"
/// @resolution.access source=inside root=inside
/// @generic.instantiation id="multiply<\"static\" & \"local\">" template=multiply arguments=("static" & "local")
/// @generic.instance id="multiply<\"bound0\" & \"local\">" template=multiply arguments=("bound0" & "local")
"#,
    );

    // reject the operator outside the declaring module
    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Force } from "./force.tspp";

declare const force: Force;
const scaled = force * 2.0;

=== dir ===
import { Force } from "./force.tspp";

declare const force: Force;
/// @type.symbol symbol=force source=force type=force.Force
/// @resolution.pattern source=force kind=binding target=force
/// @resolution.name source=Force target=force.Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=<error>
/// @resolution.pattern source=scaled kind=binding target=scaled
/// @resolution.name source=force target=force
/// @resolution.rejected source="force * 2.0"
/// @resolution.place source=force placement="local" lifetime="static" access="immutable"
/// @resolution.access source=force root=force
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '*' is not defined for 'Force' and '2'"
/// @diagnostic.label line=5 column=22 span="*" line_source="const scaled = force * 2.0;"
"#,
    );
}

#[test]
fn test_using_module_extension_overloads_foreign_type() {
    let session = TestSession::builder()
        .module(
            "force.tspp",
            r#"
export struct Force {
    value: float64;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Multiply } from "tspp:ops";
import { Force } from "./force.tspp";

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const force: Force;
const scaled = force * 2.0;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Multiply } from "tspp:ops";

import { Force } from "./force.tspp";

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const force: Force;
const scaled: Force = force * 2.0;

=== dir ===
import { Multiply } from "tspp:ops";
import { Force } from "./force.tspp";

extension of Force implements Multiply<float64> {
/// @generic.instance id=Multiply<float64> template=Multiply arguments=(float64)
/// @definition.extension symbol=<module>#2 form=local target=force.Force
/// @definition.implements symbol=<module>#2 source=Multiply<float64> target=Multiply<float64>
/// @definition.associated.type symbol=Output source="type Output = Force" key=Output value=force.Force
/// @definition.method symbol=multiply slot=multiply type=<multiply.'a>(this: &multiply.'a readonly force.Force, float64) => force.Force
/// @definition.conformance symbol=<module>#2 member=Output requirement=Multiply.Output
/// @definition.conformance symbol=<module>#2 member=multiply requirement=Multiply.multiply
/// @resolution.name source=Force target=force.Force
/// @resolution.name source=Multiply target=Multiply

    type Output = Force;
    /// @type.symbol symbol=Output source="type Output = Force" type=force.Force
    /// @resolution.name source=Force target=force.Force

    multiply(other: float64): Force {
    /// @generic.template symbol=multiply parent=template#0 parameters=('a)
    /// @type.symbol symbol=multiply type=<multiply.'a>(this: &multiply.'a readonly force.Force, float64) => force.Force
    /// @type.symbol symbol=multiply.this type=&multiply.'a readonly force.Force
    /// @type.symbol symbol=multiply.other source="other: float64" type=float64
    /// @resolution.name source=Force target=force.Force

        Force { value: this.value * other }
        /// @resolution.name source=Force target=force.Force
        /// @resolution.member source=this.value receiver=&multiply.'a readonly force.Force type=float64 kind=field target_receiver=&multiply.'a readonly force.Force key=value target=force.Force.value target_type=float64
        /// @resolution.operator source="this.value * other" type=float64 operator="*" kind=builtin operands=[this.value as float64 families=(float), other as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&multiply.'a readonly force.Force
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
/// @type.symbol symbol=force source=force type=force.Force
/// @resolution.pattern source=force kind=binding target=force
/// @resolution.name source=Force target=force.Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=force.Force
/// @resolution.pattern source=scaled kind=binding target=scaled
/// @resolution.name source=force target=force
/// @resolution.operator source="force * 2.0" type=force.Force operator="*" kind=call parameters=(float64) arguments=(provided(2.0) as float64) return=force.Force regions=("static" & "local") kind=symbol target=multiply receiver=force.Force adjustments=(borrow(&'static readonly force.Force)) instance="force.Force.<extension#1>.multiply<\"static\" & \"local\">"
/// @resolution.place source=force placement="local" lifetime="static" access="immutable"
/// @resolution.access source=force root=force
/// @generic.instantiation id="multiply<\"static\" & \"local\">" template=multiply arguments=("static" & "local")
/// @generic.instance id="multiply<\"bound0\" & \"local\">" template=multiply arguments=("bound0" & "local")
"#,
    );
}

#[test]
fn test_imported_named_extension_overloads_foreign_type() {
    let session = TestSession::builder()
        .module(
            "force.tspp",
            r#"
export struct Force {
    value: float64;
}
"#,
        )
        .module(
            "scaling.tspp",
            r#"
import { Multiply } from "tspp:ops";
import { Force } from "./force.tspp";

export extension Scaling of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Force } from "./force.tspp";
import { Scaling } from "./scaling.tspp";

declare const force: Force;
const scaled = force * 2.0;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Force } from "./force.tspp";
import { Scaling } from "./scaling.tspp";

declare const force: Force;
const scaled: Force = force * 2.0;

=== dir ===
import { Force } from "./force.tspp";
import { Scaling } from "./scaling.tspp";

declare const force: Force;
/// @type.symbol symbol=force source=force type=force.Force
/// @resolution.pattern source=force kind=binding target=force
/// @resolution.name source=Force target=force.Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=force.Force
/// @resolution.pattern source=scaled kind=binding target=scaled
/// @resolution.name source=force target=force
/// @resolution.operator source="force * 2.0" type=force.Force operator="*" kind=call parameters=(float64) arguments=(provided(2.0) as float64) return=force.Force regions=("static" & "local") kind=symbol target=scaling.Scaling.multiply receiver=force.Force adjustments=(borrow(&'static readonly force.Force)) instance="scaling.Scaling.multiply<\"static\" & \"local\">"
/// @resolution.place source=force placement="local" lifetime="static" access="immutable"
/// @resolution.access source=force root=force
/// @generic.instantiation id="scaling.Scaling.multiply<\"static\" & \"local\">" template=scaling.Scaling.multiply arguments=("static" & "local")
/// @generic.instance id="scaling.Scaling.multiply<\"bound0\" & \"local\">" template=scaling.Scaling.multiply arguments=("bound0" & "local")
"#,
    );
}
