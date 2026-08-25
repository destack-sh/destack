use crate::tests::{DirRows, TestSession};

#[test]
fn test_overloaded_negate_selects_extension_method() {
    let session = TestSession::single(
        r#"
import { Negate } from "destack:ops";

struct Charge {
    value: float64;
}

extension of Charge implements Negate {
    type Output = Charge;

    negate(&readonly this): Charge {
        Charge { value: -this.value }
    }
}

declare const charge: Charge;
const flipped = -charge;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Negate } from "destack:ops";

struct Charge {
    value: float64;
}

extension of Charge implements Negate {
    type Output = Charge;

    negate(&readonly this): Charge {
        Charge { value: -this.value }
    }
}

declare const charge: Charge;
const flipped: Charge = -charge;

=== dir ===
import { Negate } from "destack:ops";

struct Charge {
/// @type.symbol symbol=Charge type=Charge
/// @definition.struct symbol=Charge
/// @definition.field symbol=Charge.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Charge.value source="value: float64" type=float64

}

extension of Charge implements Negate {
/// @definition.extension symbol=<module>#2 form=local target=Charge
/// @definition.implements symbol=<module>#2 source=Negate target=Negate
/// @definition.associated.type symbol=Output source="type Output = Charge" key=Output value=Charge
/// @definition.method symbol=negate slot=negate type=<negate.'a, negate.P1: Place>(this: Borrowed<Charge, negate.'a & negate.P1, "readonly">) => Charge
/// @definition.conformance symbol=<module>#2 member=Output requirement=Negate.Output
/// @definition.conformance symbol=<module>#2 member=negate requirement=Negate.negate
/// @resolution.name source=Charge target=Charge
/// @resolution.name source=Negate target=Negate

    type Output = Charge;
    /// @type.symbol symbol=Output source="type Output = Charge" type=Charge
    /// @resolution.name source=Charge target=Charge

    negate(&readonly this): Charge {
    /// @generic.template symbol=negate parent=template#0 parameters=('a, P1: Place)
    /// @type.symbol symbol=negate type=<negate.'a, negate.P1: Place>(this: Borrowed<Charge, negate.'a & negate.P1, "readonly">) => Charge
    /// @type.symbol symbol=negate.this source="&readonly this" type=Borrowed<this, negate.'a & negate.P1, "readonly">
    /// @resolution.name source=Charge target=Charge

        Charge { value: -this.value }
        /// @resolution.name source=Charge target=Charge
        /// @resolution.operator source=-this.value type=float64 operator="-" kind=builtin operands=[this.value as float64 families=(float)]
        /// @resolution.member source=this.value receiver=Borrowed<Charge, negate.'a & negate.P1, "readonly"> type=float64 kind=field target_receiver=Borrowed<Charge, negate.'a & negate.P1, "readonly"> key=value target=Charge.value target_type=float64
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Charge, negate.'a & negate.P1, "readonly">
        /// @resolution.place source=this placement=negate.P1 lifetime=negate.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=negate.P1 lifetime=negate.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

declare const charge: Charge;
/// @type.symbol symbol=charge source=charge type=Charge
/// @resolution.pattern source=charge kind=binding target=charge
/// @resolution.name source=Charge target=Charge

const flipped = -charge;
/// @type.symbol symbol=flipped source=flipped type=Charge
/// @resolution.pattern source=flipped kind=binding target=flipped
/// @resolution.operator source=-charge type=Charge operator="-" kind=call parameters=() return=Charge kind=symbol target=negate receiver=Charge adjustments=(borrow(&'static readonly constant Charge)) instance="Charge.<extension#1>.negate<\"constant\">"
/// @generic.instantiation id="negate<\"constant\">" template=negate arguments=("constant")
/// @generic.instance id="negate<\"constant\">" template=negate arguments=("constant")
/// @resolution.name source=charge target=charge
/// @resolution.place source=charge placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=charge root=charge
"#,
    );
}
