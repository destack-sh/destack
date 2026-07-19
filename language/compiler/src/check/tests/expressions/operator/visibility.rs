use crate::tests::{DirRows, TestSession};

#[test]
fn test_exported_extension_overloads_imported_type() {
    let session = TestSession::builder()
        .module(
            "force.ds",
            r#"
import { Multiply } from "destack:ops";

export struct Force {
    value: float64;
}

export extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Force } from "./force.ds";

declare const force: Force;
const scaled = force * 2.0;
"#,
        )
        .build();

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Force } from "./force.ds";

declare const force: Force;
const scaled: Force = force * 2.0;

=== checked ===
import { Force } from "./force.ds";

declare const force: Force;
/// @type.symbol symbol=force source=force type=force.Force
/// @resolution.name source=Force target=force.Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=force.Force
/// @resolution.name source=force target=force
/// @resolution.call source="force * 2.0" parameters=(float64) arguments=(provided(2.0) as float64) return=force.Force kind=symbol target=force.multiply receiver=force.Force
"#,
    );
}

#[test]
fn test_local_extension_stays_module_private() {
    let session = TestSession::builder()
        .module(
            "force.ds",
            r#"
import { Multiply } from "destack:ops";

export struct Force {
    value: float64;
}

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const inside: Force;
export const doubled = inside * 2.0;
"#,
        )
        .module(
            "main.ds",
            r#"
import { Force } from "./force.ds";

declare const force: Force;
const scaled = force * 2.0;
"#,
        )
        .build();

    // the extension resolves inside its own module
    session.assert_dir_checked(
        "force.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Multiply } from "destack:ops";

export struct Force {
    value: float64;
}

extension of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}

declare const inside: Force;
export const doubled: Force = inside * 2.0;

=== checked ===
import { Multiply } from "destack:ops";

export struct Force {
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
/// @definition.method symbol=multiply slot=multiply type=(this: this, float64) => Force
/// @resolution.name source=Force target=Force
/// @resolution.name source=Multiply target=ops.multiply.Multiply

    type Output = Force;
    /// @type.symbol symbol=Output source="type Output = Force" type=Force
    /// @resolution.name source=Force target=Force

    multiply(other: float64): Force {
    /// @type.symbol symbol=multiply type=(this: this, float64) => Force
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

declare const inside: Force;
/// @type.symbol symbol=inside source=inside type=Force
/// @resolution.name source=Force target=Force

export const doubled = inside * 2.0;
/// @type.symbol symbol=doubled source=doubled type=Force
/// @resolution.name source=inside target=inside
/// @resolution.call source="inside * 2.0" parameters=(float64) arguments=(provided(2.0) as float64) return=Force kind=symbol target=multiply receiver=Force
"#,
    );

    // other modules never see the local extension
    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Force } from "./force.ds";

declare const force: Force;
const scaled = force * 2.0;

=== checked ===
import { Force } from "./force.ds";

declare const force: Force;
/// @type.symbol symbol=force source=force type=force.Force
/// @resolution.name source=Force target=force.Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=<error>
/// @resolution.name source=force target=force
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
            "force.ds",
            r#"
export struct Force {
    value: float64;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Multiply } from "destack:ops";
import { Force } from "./force.ds";

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Multiply } from "destack:ops";

import { Force } from "./force.ds";

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
import { Force } from "./force.ds";

extension of Force implements Multiply<float64> {
/// @definition.extension symbol=<module>#2 form=local target=force.Force
/// @definition.implements symbol=<module>#2 source=Multiply<float64> target=ops.multiply.Multiply arguments=(float64)
/// @definition.associated.type symbol=Output source="type Output = Force" key=Output value=force.Force
/// @definition.method symbol=multiply slot=multiply type=(this: this, float64) => force.Force
/// @resolution.name source=Force target=force.Force
/// @resolution.name source=Multiply target=ops.multiply.Multiply

    type Output = Force;
    /// @type.symbol symbol=Output source="type Output = Force" type=force.Force
    /// @resolution.name source=Force target=force.Force

    multiply(other: float64): Force {
    /// @type.symbol symbol=multiply type=(this: this, float64) => force.Force
    /// @type.symbol symbol=multiply.other source="other: float64" type=float64
    /// @resolution.name source=Force target=force.Force

        Force { value: this.value * other }
        /// @resolution.name source=Force target=force.Force
        /// @resolution.member source=this.value receiver=force.Force kind=symbol target=force.Force.value
        /// @resolution.call source="this.value * other" parameters=() return=float64 kind=builtin builtin=binary.multiply
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=force.Force
        /// @resolution.name source=other target=multiply.other

    }
}

declare const force: Force;
/// @type.symbol symbol=force source=force type=force.Force
/// @resolution.name source=Force target=force.Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=force.Force
/// @resolution.name source=force target=force
/// @resolution.call source="force * 2.0" parameters=(float64) arguments=(provided(2.0) as float64) return=force.Force kind=symbol target=multiply receiver=force.Force
"#,
    );
}

#[test]
fn test_imported_named_extension_overloads_foreign_type() {
    let session = TestSession::builder()
        .module(
            "force.ds",
            r#"
export struct Force {
    value: float64;
}
"#,
        )
        .module(
            "scaling.ds",
            r#"
import { Multiply } from "destack:ops";
import { Force } from "./force.ds";

export extension Scaling of Force implements Multiply<float64> {
    type Output = Force;

    multiply(other: float64): Force {
        Force { value: this.value * other }
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Force } from "./force.ds";
import { Scaling } from "./scaling.ds";

declare const force: Force;
const scaled = force * 2.0;
"#,
        )
        .build();

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Force } from "./force.ds";
import { Scaling } from "./scaling.ds";

declare const force: Force;
const scaled: Force = force * 2.0;

=== checked ===
import { Force } from "./force.ds";
import { Scaling } from "./scaling.ds";

declare const force: Force;
/// @type.symbol symbol=force source=force type=force.Force
/// @resolution.name source=Force target=force.Force

const scaled = force * 2.0;
/// @type.symbol symbol=scaled source=scaled type=force.Force
/// @resolution.name source=force target=force
/// @resolution.call source="force * 2.0" parameters=(float64) arguments=(provided(2.0) as float64) return=force.Force kind=symbol target=scaling.Scaling.multiply receiver=force.Force
"#,
    );
}
