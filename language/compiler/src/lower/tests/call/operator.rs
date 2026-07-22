use crate::tests::TestSession;

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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Vector {
    x: int32;
    y: int32;
}

function main.Vector.add<L0: lifetime>(v0: ref<Vector, borrowed, lifetime(L0), exclusive>, v1: Vector): Vector {
entry(v0: ref<Vector, borrowed, lifetime(L0), exclusive>, v1: Vector):
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: int32 = load v2
    v4: int32 = field.get v1, 0
    v5: int32 = int.add v3, v4
    v6: ref<int32, borrowed, exclusive> = field.address v0, 1
    v7: int32 = load v6
    v8: int32 = field.get v1, 1
    v9: int32 = int.add v7, v8
    v10: Vector = aggregate (v5, v9)
    return v10
}

function main.combine(): int32 {
    local l0: Vector
    local l1: Vector
    local l2: Vector

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: Vector = aggregate (v0, v1)
    local.set l0, v2
    v3: int32 = 3
    v4: int32 = 4
    v5: Vector = aggregate (v3, v4)
    local.set l1, v5
    v6: ref<Vector, borrowed, exclusive> = local.address l0
    v7: Vector = local.get l1
    v8: Vector = call main.Vector.add(v6, v7)
    local.set l2, v8
    v9: Vector = local.get l2
    v10: int32 = field.get v9, 0
    return v10
}
/// @layout.struct name=Vector size=8 align=4
/// @layout.field owner=Vector index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Vector index=1 name=y offset=4 size=4 align=4
"#,
    );
}

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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Charge {
    amount: int32;
}

function main.Charge.negate<L0: lifetime>(v0: ref<Charge, borrowed, lifetime(L0), exclusive>): Charge {
entry(v0: ref<Charge, borrowed, lifetime(L0), exclusive>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: int32 = load v1
    v3: int32 = int.negate v2
    v4: Charge = aggregate (v3)
    return v4
}

function main.invert(): int32 {
    local l0: Charge
    local l1: Charge

entry:
    v0: int32 = 5
    v1: Charge = aggregate (v0)
    local.set l0, v1
    v2: ref<Charge, borrowed, exclusive> = local.address l0
    v3: Charge = call main.Charge.negate(v2)
    local.set l1, v3
    v4: Charge = local.get l1
    v5: int32 = field.get v4, 0
    return v5
}
/// @layout.struct name=Charge size=4 align=4
/// @layout.field owner=Charge index=0 name=amount offset=0 size=4 align=4
"#,
    );
}
