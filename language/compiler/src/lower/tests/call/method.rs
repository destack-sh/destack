use crate::tests::TestSession;

#[test]
fn test_lower_struct_method_call_through_an_exclusive_borrow() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;

    length(): int32 {
        return this.x + this.y;
    }
}

function measure(): int32 {
    let point = Point { x: 3, y: 4 };
    return point.length();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
    y: int32;
}

function main.Point.length<L0: lifetime>(v0: ref<Point, borrowed, lifetime(L0), exclusive>): int32 {
entry(v0: ref<Point, borrowed, lifetime(L0), exclusive>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, exclusive> = field.address v0, 1
    v4: int32 = load v3
    v5: int32 = int.add v2, v4
    return v5
}

function main.measure(): int32 {
    local l0: Point

entry:
    v0: int32 = 3
    v1: int32 = 4
    v2: Point = aggregate (v0, v1)
    local.set l0, v2
    v3: ref<Point, borrowed, exclusive> = local.address l0
    v4: int32 = call main.Point.length(v3)
    return v4
}
/// @layout.struct name=Point size=8 align=4 fields=(x@0+4, y@4+4)
"#,
    );
}

#[test]
fn test_lower_struct_method_mutation_through_implicit_this() {
    let session = TestSession::single(
        r#"
struct Counter {
    count: int32;

    bump(by: int32): void {
        this.count += by;
    }
}

function tally(): int32 {
    let counter = Counter { count: 0 };
    counter.bump(5);
    return counter.count;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Counter {
    count: int32;
}

function main.Counter.bump<L0: lifetime>(v0: ref<Counter, borrowed, lifetime(L0), exclusive>, v1: int32): void {
entry(v0: ref<Counter, borrowed, lifetime(L0), exclusive>, v1: int32):
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: int32 = load v2
    v4: int32 = int.add v3, v1
    v5: ref<int32, borrowed, exclusive> = field.address v0, 0
    store v5, v4
    return
}

function main.tally(): int32 {
    local l0: Counter

entry:
    v0: int32 = 0
    v1: Counter = aggregate (v0)
    local.set l0, v1
    v2: ref<Counter, borrowed, exclusive> = local.address l0
    v3: int32 = 5
    call main.Counter.bump(v2, v3)
    v4: Counter = local.get l0
    v5: int32 = field.get v4, 0
    return v5
}
/// @layout.struct name=Counter size=4 align=4 fields=(count@0+4)
"#,
    );
}

#[test]
fn test_lower_enum_method_call_over_the_variant_carrier() {
    let session = TestSession::single(
        r#"
enum Status {
    Active = 1,
    Inactive = 2,

    isActive(): boolean {
        return this == Status.Active;
    }
}

function probe(status: Status): boolean {
    return status.isActive();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Status = variant<int32, void> { 1int32 = void; 2int32 = void; };

function main.Status.isActive<L0: lifetime>(v0: ref<Status, borrowed, lifetime(L0), exclusive>): boolean {
entry(v0: ref<Status, borrowed, lifetime(L0), exclusive>):
    v1: Status = load v0
    v2: int32 = variant.tag v1
    v3: Status = variant.new 0
    v4: int32 = variant.tag v3
    v5: boolean = int.eq v2, v4
    return v5
}

function main.probe(v0: Status): boolean {
    local l0: Status

entry(v0: Status):
    local.set l0, v0
    v1: ref<Status, borrowed, exclusive> = local.address l0
    v2: boolean = call main.Status.isActive(v1)
    return v2
}
/// @layout.variant name=Status size=4 align=4 encoding=direct(tag@0+4) cases=(1@4, 2@4)
"#,
    );
}
