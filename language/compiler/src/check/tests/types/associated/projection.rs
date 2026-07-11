use crate::tests::{DirRows, TestSession};

#[test]
fn test_receiver_projections_reduce_for_generic_extensions() {
    let session = TestSession::single(
        r#"
import { Numeric } from "destack:math";

interface Doubles {
    type Output;
    double(): this.Output;
}

struct Pair<T: Numeric> {
    x: T;
}

extension<T: Numeric> of Pair<T> implements Doubles {
    type Output = Pair<T>;

    double(): this.Output {
        Pair { x: this.x + this.x }
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
import { Numeric } from "destack:math";

interface Doubles {
    type Output;
    double(): this.Output;
}

struct Pair<out T: Numeric> {
    x: T;
}

extension<T: Numeric> of Pair<T> implements Doubles {
    type Output = Pair<T>;

    double(): Pair<T>.Output {
        Pair<T> { x: this.x + this.x }
    }
}

=== checked ===
import { Numeric } from "destack:math";

interface Doubles {
/// @type.symbol symbol=Doubles type=Doubles
/// @definition.interface symbol=Doubles
/// @definition.associated.type symbol=Doubles.Output source="type Output" key=Output
/// @definition.method symbol=Doubles.double source="double(): this.Output" slot=double type=(this: Doubles) => this.Output

    type Output;
    double(): this.Output;
    /// @type.symbol symbol=Doubles.double source="double(): this.Output" type=(this: Doubles) => this.Output

}

struct Pair<T: Numeric> {
/// @generic.template symbol=Pair parameters=(out T#1: math.scalar.Numeric)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=(out T#1: math.scalar.Numeric)
/// @definition.field symbol=Pair.x source="x: T" key=x type=T#1
/// @type.symbol symbol=Pair.T source="T: Numeric" type=T#1
/// @resolution.name source=Numeric target=math.scalar.Numeric

    x: T;
    /// @type.symbol symbol=Pair.x source="x: T" type=T#1
    /// @resolution.name source=T target=Pair.T

}

extension<T: Numeric> of Pair<T> implements Doubles {
/// @generic.template symbol=<module>#2 parameters=(T#2: math.scalar.Numeric)
/// @definition.extension symbol=<module>#2 form=local target=Pair<T#2>
/// @definition.implements symbol=<module>#2 source=Doubles target=Doubles
/// @definition.associated.type symbol=Output source="type Output = Pair<T>" key=Output value=Pair<T#2>
/// @definition.method symbol=double slot=double type=(this: Pair<T#2>) => Pair<T#2>.Output
/// @type.symbol symbol=T source="T: Numeric" type=T#2
/// @resolution.name source=Numeric target=math.scalar.Numeric
/// @resolution.name source=Pair target=Pair
/// @resolution.name source=T target=T
/// @resolution.name source=Doubles target=Doubles

    type Output = Pair<T>;
    /// @type.symbol symbol=Output source="type Output = Pair<T>" type=Pair<T#2>
    /// @resolution.name source=Pair target=Pair
    /// @resolution.name source=T target=T

    double(): this.Output {
    /// @type.symbol symbol=double type=(this: Pair<T#2>) => Pair<T#2>.Output reduced=(this: Pair<T#2>) => Pair<T#2>

        Pair { x: this.x + this.x }
        /// @type.node source="Pair { x: this.x + this.x }" type=Pair<T#2>
        /// @resolution.name source=Pair target=Pair
        /// @generic.instance source="Pair { x: this.x + this.x }" id=Pair<T#2>
        /// @type.node source="this.x + this.x" type=T#2
        /// @type.node source=this.x type=T#2
        /// @resolution.member source=this.x receiver=Pair<T#2> kind=symbol target=Pair.x
        /// @resolution.call source="this.x + this.x" parameters=() return=T#2 kind=builtin builtin=binary.add
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Pair<T#2>
        /// @type.node source=this.x type=T#2
        /// @resolution.member source=this.x receiver=Pair<T#2> kind=symbol target=Pair.x
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Pair<T#2>

    }
}

/// @generic.instance id=Pair<T#2> template=Pair arguments=(T#2)
"#,
        r#""#,
    );
}
