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
@copy
type test.main.Vector {
    x: int32;
    y: int32;
}

function test.main.Vector.add<'a>(v0: ref<test.main.Vector, borrowed, 'a, readonly, local>, v1: test.main.Vector): test.main.Vector {
    local l0: test.main.Vector
    local l1: ref<test.main.Vector, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Vector, borrowed, 'a, readonly, local>, v1: test.main.Vector):
    local.set l0, v1
    local.set l1, v0
    v2: ref<test.main.Vector, borrowed, 'a, readonly, local> = local.get l1
    v3: ref<int32, borrowed, 'a, readonly, local> = field.project v2, 0
    v4: int32 = load v3
    v5: test.main.Vector = local.get l0
    v6: int32 = field.get v5, 0
    v7: int32 = add v4, v6
    v8: ref<test.main.Vector, borrowed, 'a, readonly, local> = local.get l1
    v9: ref<int32, borrowed, 'a, readonly, local> = field.project v8, 1
    v10: int32 = load v9
    v11: test.main.Vector = local.get l0
    v12: int32 = field.get v11, 1
    v13: int32 = add v10, v12
    v14: test.main.Vector = aggregate (v7, v13)
    return v14
}

/// @layout.struct name=test.main.Vector size=8 align=4
/// @layout.field owner=test.main.Vector index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Vector index=1 name=y offset=4 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.combine", r#"
@copy
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
    local.set l0, v2
    v3: int32 = 3
    v4: int32 = 4
    v5: test.main.Vector = aggregate (v3, v4)
    local.set l1, v5
    v6: test.main.Vector = local.get l1
    v7: ref<test.main.Vector, borrowed, 'frame, readonly, local> = local.address l0
    v8: test.main.Vector = call test.main.Vector.add(v7, v6): <'a>(ref<test.main.Vector, borrowed, 'a, readonly, local>, test.main.Vector) => test.main.Vector
    local.set l2, v8
    v9: test.main.Vector = local.get l2
    v10: int32 = field.get v9, 0
    return v10
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
@copy
type test.main.Charge {
    amount: int32;
}

function test.main.Charge.negate<'a>(v0: ref<test.main.Charge, borrowed, 'a, readonly, local>): test.main.Charge {
    local l0: ref<test.main.Charge, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Charge, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Charge, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<int32, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    v4: int32 = negate v3
    v5: test.main.Charge = aggregate (v4)
    return v5
}

/// @layout.struct name=test.main.Charge size=4 align=4
/// @layout.field owner=test.main.Charge index=0 name=amount offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.invert", r#"
@copy
type test.main.Charge {
    amount: int32;
}

function test.main.invert(): int32 {
    local l0: test.main.Charge
    local l1: test.main.Charge

entry:
    v0: int32 = 5
    v1: test.main.Charge = aggregate (v0)
    local.set l0, v1
    v2: ref<test.main.Charge, borrowed, 'frame, readonly, local> = local.address l0
    v3: test.main.Charge = call test.main.Charge.negate(v2): <'a>(ref<test.main.Charge, borrowed, 'a, readonly, local>) => test.main.Charge
    local.set l1, v3
    v4: test.main.Charge = local.get l1
    v5: int32 = field.get v4, 0
    return v5
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

function isBefore<T: PartialCompare<T>>(left: &readonly T, right: &readonly T): boolean {
    return *left < *right;
}

function isAtMost<T: Compare<T>>(left: &readonly T, right: &readonly T): boolean {
    return *left <= *right;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.isAtMost", r#"
@copy
@languageItem("ops.Ordering")
type Ordering;

@languageItem("ops.Compare")
type Compare<T>;

function test.main.isAtMost<T: Compare<T>, 'a, 'b>(v0: ref<T, borrowed, 'a, readonly, local>, v1: ref<T, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<T, borrowed, 'a, readonly, local>
    local l1: ref<T, borrowed, 'b, readonly, local>

entry(v0: ref<T, borrowed, 'a, readonly, local>, v1: ref<T, borrowed, 'b, readonly, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<T, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<T, borrowed, 'b, readonly, local> = local.get l1
    v4: ref<T, borrowed, 'b, readonly, local> = cast.bit v3 -> ref<T, borrowed, 'b, readonly, local>
    v5: ref<T, borrowed, 'a, readonly, local> = cast.bit v2 -> ref<T, borrowed, 'a, readonly, local>
    v6: Ordering = call.witness T, Compare<T>, Compare.compare(v5, v4): <'a, 'b>(ref<T, borrowed, 'a, readonly, local>, ref<T, borrowed, 'b, readonly, local>) => Ordering
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
@languageItem("ops.PartialCompare")
type PartialCompare<T>;

@copy
@languageItem("ops.Ordering")
type Ordering;

function test.main.isBefore<T: PartialCompare<T>, 'a, 'b>(v0: ref<T, borrowed, 'a, readonly, local>, v1: ref<T, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<T, borrowed, 'a, readonly, local>
    local l1: ref<T, borrowed, 'b, readonly, local>
    local l2: boolean, readonly
    local l3: variant<uint1> { 0uint1 = Ordering; 1uint1 = null; }

entry(v0: ref<T, borrowed, 'a, readonly, local>, v1: ref<T, borrowed, 'b, readonly, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<T, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<T, borrowed, 'b, readonly, local> = local.get l1
    v4: ref<T, borrowed, 'b, readonly, local> = cast.bit v3 -> ref<T, borrowed, 'b, readonly, local>
    v5: ref<T, borrowed, 'a, readonly, local> = cast.bit v2 -> ref<T, borrowed, 'a, readonly, local>
    v6: variant<uint1> { 0uint1 = Ordering; 1uint1 = null; } = call.witness T, PartialCompare<T>, PartialCompare.partialCompare(v5, v4): <'a, 'b>(ref<T, borrowed, 'a, readonly, local>, ref<T, borrowed, 'b, readonly, local>) => variant<uint1> { 0uint1 = Ordering; 1uint1 = null; }
    local.set l3, v6
    v7: variant<uint1> { 0uint1 = Ordering; 1uint1 = null; } = local.get l3
    variant.switch v7, 0 => b1, else b2

b1:
    v8: Ordering = variant.payload v6, 0
    v9: int8 = variant.tag v8
    v10: int8 = -1
    v11: boolean = eq v9, v10
    local.set l2, v11
    jump b3

b2:
    v12: boolean = false
    local.set l2, v12
    jump b3

b3:
    v13: boolean = local.get l2
    return v13
}

/// @layout.variant name=type@18 size=1 align=1
/// @layout.discriminant owner=type@18 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@18 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@18 index=1 discriminant=1 payload_offset=0
"#,
    );
}
