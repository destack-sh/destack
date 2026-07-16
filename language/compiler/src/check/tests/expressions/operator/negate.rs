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

    negate(): Charge {
        Charge { value: -this.value }
    }
}

declare const charge: Charge;
const flipped = -charge;
"#,
    );

    session.assert_dir_checked(
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

    negate(): Charge {
        Charge { value: -this.value }
    }
}

declare const charge: Charge;
const flipped: Charge = -charge;

=== checked ===
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
/// @definition.implements symbol=<module>#2 source=Negate target=ops.negate.Negate
/// @definition.associated.type symbol=Output source="type Output = Charge" key=Output value=Charge
/// @definition.method symbol=negate slot=negate type=(this: this) => Charge
/// @resolution.name source=Charge target=Charge
/// @resolution.name source=Negate target=ops.negate.Negate

    type Output = Charge;
    /// @type.symbol symbol=Output source="type Output = Charge" type=Charge
    /// @resolution.name source=Charge target=Charge

    negate(): Charge {
    /// @type.symbol symbol=negate type=(this: this) => Charge
    /// @resolution.name source=Charge target=Charge

        Charge { value: -this.value }
        /// @resolution.name source=Charge target=Charge
        /// @resolution.call source=-this.value parameters=() return=float64 kind=builtin builtin=unary.negate
        /// @resolution.member source=this.value receiver=Charge kind=symbol target=Charge.value
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Charge

    }
}

declare const charge: Charge;
/// @type.symbol symbol=charge source=charge type=Charge
/// @resolution.name source=Charge target=Charge

const flipped = -charge;
/// @type.symbol symbol=flipped source=flipped type=Charge
/// @resolution.call source=-charge parameters=() return=Charge kind=symbol target=negate receiver=Charge
/// @resolution.name source=charge target=charge
"#,
    );
}
