use crate::tests::TestSession;

/// Builtin operators on parameter operands lower over the parameter representation, the instance
/// deciding the scalar format or identity.
#[test]
fn test_lower_builtin_operators_over_parameter_operands() {
    let session = TestSession::single(
        r#"
import { Float, Numeric } from "tspp:math";
import { StrictEqual } from "tspp:ops";

function identical<T: StrictEqual<T>>(a: T, b: T): boolean {
    a === b
}

function widen<T: int32 | int64>(a: T, b: T): T {
    a + b
}

function nearly<T: Float>(a: T, b: T): boolean {
    a == b
}

function scaled<T: Float>(a: T, b: T): T {
    a * b
}

function origin<T: Numeric>(): T {
    T.zero()
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.identical",
        r#"
function test.main.identical<T: StrictEqual<T>>(v0: T, v1: T): boolean {
    local l0: T
    local l1: T

entry(v0: T, v1: T):
    store l0, v0
    store l1, v1
    v2: T = load l0
    v3: T = load l1
    v4: boolean = eq v2, v3
    return v4
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.widen",
        r#"
function test.main.widen<T: Copy>(v0: T, v1: T): T {
    local l0: T
    local l1: T

entry(v0: T, v1: T):
    store l0, v0
    store l1, v1
    v2: T = load l0
    v3: T = load l1
    v4: T = add v2, v3
    return v4
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.nearly",
        r#"
function test.main.nearly<T: Float>(v0: T, v1: T): boolean {
    local l0: T
    local l1: T

entry(v0: T, v1: T):
    store l0, v0
    store l1, v1
    v2: T = load l0
    v3: T = load l1
    v4: boolean = eq v2, v3
    return v4
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.origin",
        r#"
@nocopy
@languageItem("math.Zero")
type Zero;

function test.main.origin<T: Concrete & Copy & Clone & Zero & One>(): T {
entry:
    v0: T = call.witness T, Zero, Zero.zero(): () => T
    return v0
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.scaled",
        r#"
function test.main.scaled<T: Float>(v0: T, v1: T): T {
    local l0: T
    local l1: T

entry(v0: T, v1: T):
    store l0, v0
    store l1, v1
    v2: T = load l0
    v3: T = load l1
    v4: T = mul v2, v3
    return v4
}
"#,
    );
}
