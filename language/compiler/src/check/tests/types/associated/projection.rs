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

    session.assert_dir_checked(
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

    double(): this.Output {
        Pair<T> { x: this.x + this.x }
    }
}

=== checked ===
import { Numeric } from "destack:math";

interface Doubles {
/// @type.symbol symbol=Doubles type=Doubles
/// @definition.interface symbol=Doubles
/// @definition.associated.type symbol=Doubles.Output source="type Output" key=Output
/// @definition.method symbol=Doubles.double source="double(): this.Output" slot=double type=(this: this) => this.Output

    type Output;
    double(): this.Output;
    /// @type.symbol symbol=Doubles.double source="double(): this.Output" type=(this: this) => this.Output

}

struct Pair<T: Numeric> {
/// @generic.template symbol=Pair parameters=(out T#1: math.numeric.Numeric)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=(out T#1: math.numeric.Numeric)
/// @definition.field symbol=Pair.x source="x: T" key=x type=T#1
/// @type.symbol symbol=Pair.T source="T: Numeric" type=T#1
/// @resolution.name source=Numeric target=math.numeric.Numeric

    x: T;
    /// @type.symbol symbol=Pair.x source="x: T" type=T#1
    /// @resolution.name source=T target=Pair.T

}

extension<T: Numeric> of Pair<T> implements Doubles {
/// @generic.template symbol=<module>#2 parameters=(T#2: math.numeric.Numeric)
/// @definition.extension symbol=<module>#2 form=local target=Pair<T#2>
/// @definition.implements symbol=<module>#2 source=Doubles target="Doubles<type Output = Pair<T#2>>"
/// @definition.associated.type symbol=Output source="type Output = Pair<T>" key=Output value=Pair<T#2>
/// @definition.method symbol=double slot=double type=<double.'l0>(this: &double.'l0 exclusive this) => this.Output
/// @type.symbol symbol=T source="T: Numeric" type=T#2
/// @resolution.name source=Numeric target=math.numeric.Numeric
/// @resolution.name source=Pair target=Pair
/// @resolution.name source=T target=T
/// @resolution.name source=Doubles target=Doubles

    type Output = Pair<T>;
    /// @type.symbol symbol=Output source="type Output = Pair<T>" type=Pair<T#2>
    /// @resolution.name source=Pair target=Pair
    /// @resolution.name source=T target=T

    double(): this.Output {
    /// @generic.template symbol=double parent=template#2 parameters=('l0)
    /// @type.symbol symbol=double type=<double.'l0>(this: &double.'l0 exclusive this) => this.Output

        Pair { x: this.x + this.x }
        /// @type.node source="Pair { x: this.x + this.x }" type=Pair<T#2>
        /// @resolution.name source=Pair target=Pair
        /// @generic.instance source="Pair { x: this.x + this.x }" id=Pair<T#2>
        /// @type.node source="this.x + this.x" type=T#2
        /// @type.node source=this.x type=T#2
        /// @resolution.member source=this.x receiver=&double.'l0 exclusive Pair<T#2> type=T#2 kind=field target_receiver=&double.'l0 exclusive Pair<T#2> key=x target=Pair.x target_type=T#2
        /// @resolution.operator source="this.x + this.x" type=T#2 operator="+" kind=builtin operands=[this.x as T#2 families=(integer | float), this.x as T#2 families=(integer | float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&double.'l0 exclusive Pair<T#2>
        /// @resolution.place source=this placement="local" lifetime=double.'l0 access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement="local" lifetime=double.'l0 access="exclusive"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @type.node source=this.x type=T#2
        /// @resolution.member source=this.x receiver=&double.'l0 exclusive Pair<T#2> type=T#2 kind=field target_receiver=&double.'l0 exclusive Pair<T#2> key=x target=Pair.x target_type=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&double.'l0 exclusive Pair<T#2>
        /// @resolution.place source=this placement="local" lifetime=double.'l0 access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement="local" lifetime=double.'l0 access="exclusive"
        /// @resolution.access source=this.x root=this keys=[x]

    }
}

/// @generic.instance id=Pair<T#2> template=Pair arguments=(T#2)
"#,
    );
}
