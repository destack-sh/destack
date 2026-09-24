use crate::tests::TestSession;

/// An overloaded plus lowers to a call of the implemented add method.
#[test]
fn test_lower_overloaded_plus_through_the_add_method() {
    let session = TestSession::single(
        r#"
struct Vector implements Add<Vector> {
    x: int32;
    y: int32;

    type Output = Vector;

    add(other: Vector): Vector {
        return Vector {
            x: this.x + other.x,
            y: this.y + other.y,
        };
    }
}

function combine(): int32 {
    let left = Vector { x: 1, y: 2 };
    let right = Vector { x: 3, y: 4 };
    let sum = left + right;
    return sum.x;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Vector.add", r#"
type test.main.Vector {
    x: int32;
    y: int32;
}

function test.main.Vector.add<'a>(v0: ref<test.main.Vector, borrowed, 'a, readonly>, v1: test.main.Vector): test.main.Vector {
    local l0: test.main.Vector
    local l1: ref<test.main.Vector, borrowed, 'a, readonly>

entry(v0: ref<test.main.Vector, borrowed, 'a, readonly>, v1: test.main.Vector):
    store l0, v1
    store l1, v0
    v2: ref<test.main.Vector, borrowed, 'a, readonly> = load l1
    v3: int32 = load (*v2).0
    v4: int32 = load (l0).0
    v5: int32 = add v3, v4
    v6: ref<test.main.Vector, borrowed, 'a, readonly> = load l1
    v7: int32 = load (*v6).1
    v8: int32 = load (l0).1
    v9: int32 = add v7, v8
    v10: test.main.Vector = aggregate (v5, v9)
    return v10
}

/// @layout.struct name=test.main.Vector size=8 align=4
/// @layout.field owner=test.main.Vector index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Vector index=1 name=y offset=4 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.combine", r#"
type test.main.Vector {
    x: int32;
    y: int32;
}

function test.main.combine(): int32 {
    local l0: test.main.Vector
    local l1: test.main.Vector
    local l2: test.main.Vector

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: test.main.Vector = aggregate (v0, v1)
    store l0, v2
    v3: int32 = 3
    v4: int32 = 4
    v5: test.main.Vector = aggregate (v3, v4)
    store l1, v5
    v6: test.main.Vector = load l1
    v7: ref<test.main.Vector, borrowed, 'frame, readonly> = address l0
    v8: test.main.Vector = call test.main.Vector.add(v7, v6): (ref<test.main.Vector, borrowed, 'frame, readonly>, test.main.Vector) => test.main.Vector
    store l2, v8
    v9: int32 = load (l2).0
    return v9
}

/// @layout.struct name=test.main.Vector size=8 align=4
/// @layout.field owner=test.main.Vector index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Vector index=1 name=y offset=4 size=4 align=4
"#);
}

/// An overloaded unary minus lowers to a call of the implemented negate method.
#[test]
fn test_lower_overloaded_negate_through_the_negate_method() {
    let session = TestSession::single(
        r#"
struct Charge implements Negate {
    amount: int32;

    type Output = Charge;

    negate(): Charge {
        return Charge { amount: -this.amount };
    }
}

function invert(): int32 {
    let charge = Charge { amount: 5 };
    let flipped = -charge;
    return flipped.amount;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Charge.negate",
        r#"
type test.main.Charge {
    amount: int32;
}

function test.main.Charge.negate<'a>(v0: ref<test.main.Charge, borrowed, 'a, readonly>): test.main.Charge {
    local l0: ref<test.main.Charge, borrowed, 'a, readonly>

entry(v0: ref<test.main.Charge, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Charge, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    v3: int32 = negate v2
    v4: test.main.Charge = aggregate (v3)
    return v4
}

/// @layout.struct name=test.main.Charge size=4 align=4
/// @layout.field owner=test.main.Charge index=0 name=amount offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.invert", r#"
type test.main.Charge {
    amount: int32;
}

function test.main.invert(): int32 {
    local l0: test.main.Charge
    local l1: test.main.Charge

entry:
    v0: int32 = 5
    v1: test.main.Charge = aggregate (v0)
    store l0, v1
    v2: ref<test.main.Charge, borrowed, 'frame, readonly> = address l0
    v3: test.main.Charge = call test.main.Charge.negate(v2): (ref<test.main.Charge, borrowed, 'frame, readonly>) => test.main.Charge
    store l1, v3
    v4: int32 = load (l1).0
    return v4
}

/// @layout.struct name=test.main.Charge size=4 align=4
/// @layout.field owner=test.main.Charge index=0 name=amount offset=0 size=4 align=4
"#);
}

/// Comparing through dereferenced borrows reborrows the operands for the protocol call.
#[test]
fn test_compare_through_dereferenced_borrows_by_reborrowing_them() {
    let session = TestSession::single(
        r#"
import { Compare, PartialCompare } from "destack:ops";

function isBefore<T: PartialCompare<T>>(left: &immutable T, right: &immutable T): boolean {
    return *left < *right;
}

function isAtMost<T: Compare<T>>(left: &immutable T, right: &immutable T): boolean {
    return *left <= *right;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.isAtMost", r#"
@languageItem("ops.Ordering")
type Ordering;

@nocopy
@languageItem("ops.Compare")
type Compare<T>;

function test.main.isAtMost<T: Compare<T>, 'a, 'b>(v0: ref<?T, borrowed, 'a, immutable>, v1: ref<?T, borrowed, 'b, immutable>): boolean {
    local l0: ref<?T, borrowed, 'a, immutable>
    local l1: ref<?T, borrowed, 'b, immutable>

entry(v0: ref<?T, borrowed, 'a, immutable>, v1: ref<?T, borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<?T, borrowed, 'a, immutable> = load l0
    v3: ref<?T, borrowed, 'b, immutable> = load l1
    v4: ref<?T, borrowed, 'b, immutable> = address (*v3)
    v5: ref<?T, borrowed, 'a, immutable> = address (*v2)
    v6: Ordering = call.witness T, Compare<T>, Compare.compare(v5, v4): (ref<?T, borrowed, 'a, immutable>, ref<?T, borrowed, 'b, immutable>) => Ordering
    v7: int8 = variant.tag v6
    v8: int8 = 1
    v9: boolean = ne v7, v8
    return v9
}
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.isBefore",
        r#"
@nocopy
@languageItem("ops.PartialCompare")
type PartialCompare<T>;

@languageItem("ops.Ordering")
type Ordering;

function test.main.isBefore<T: PartialCompare<T>, 'a, 'b>(v0: ref<?T, borrowed, 'a, immutable>, v1: ref<?T, borrowed, 'b, immutable>): boolean {
    local l0: ref<?T, borrowed, 'a, immutable>
    local l1: ref<?T, borrowed, 'b, immutable>
    local l2: boolean, readonly
    local l3: variant<uint1> { 0uint1 = Ordering; 1uint1 = null; }

entry(v0: ref<?T, borrowed, 'a, immutable>, v1: ref<?T, borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<?T, borrowed, 'a, immutable> = load l0
    v3: ref<?T, borrowed, 'b, immutable> = load l1
    v4: ref<?T, borrowed, 'b, immutable> = address (*v3)
    v5: ref<?T, borrowed, 'a, immutable> = address (*v2)
    v6: variant<uint1> { 0uint1 = Ordering; 1uint1 = null; } = call.witness T, PartialCompare<T>, PartialCompare.partialCompare(v5, v4): (ref<?T, borrowed, 'a, immutable>, ref<?T, borrowed, 'b, immutable>) => variant<uint1> { 0uint1 = Ordering; 1uint1 = null; }
    store l3, v6
    v7: uint1 = variant.tag.load l3
    switch v7, b2, 0 => b1

b1:
    v8: Ordering = variant.payload v6, 0
    v9: int8 = variant.tag v8
    v10: int8 = -1
    v11: boolean = eq v9, v10
    store l2, v11
    jump b3

b2:
    v12: boolean = false
    store l2, v12
    jump b3

b3:
    v13: boolean = load l2
    return v13
}
"#,
    );
}
