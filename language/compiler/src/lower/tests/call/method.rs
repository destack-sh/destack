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

function test.main.Point.length<'a>(v0: ref<Point, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<Point, borrowed, 'a, readonly, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, readonly, local> = field.address v0, 1
    v4: int32 = load v3
    v5: int32 = add v2, v4
    return v5
}

function test.main.measure(): int32 {
    local l0: Point

entry:
    v0: int32 = 3
    v1: int32 = 4
    v2: Point = aggregate (v0, v1)
    local.set l0, v2
    v3: ref<Point, borrowed, 'frame, readonly, local> = local.address l0
    v4: int32 = call test.main.Point.length(v3): <'a>(ref<Point, borrowed, 'a, readonly, local>) => int32
    return v4
}

/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_struct_method_mutation_through_implicit_this() {
    let session = TestSession::single(
        r#"
struct Counter {
    count: int32;

    bump(&exclusive this, by: int32): void {
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

function test.main.Counter.bump<'a>(v0: ref<Counter, borrowed, 'a, exclusive, local>, v1: int32): void {
entry(v0: ref<Counter, borrowed, 'a, exclusive, local>, v1: int32):
    v2: ref<int32, borrowed, exclusive, local> = field.address v0, 0
    v3: int32 = load v2
    v4: int32 = add v3, v1
    v5: ref<int32, borrowed, exclusive, local> = field.address v0, 0
    store v5, v4
    return
}

function test.main.tally(): int32 {
    local l0: Counter

entry:
    v0: int32 = 0
    v1: Counter = aggregate (v0)
    local.set l0, v1
    v2: ref<Counter, borrowed, 'frame, exclusive, local> = local.address l0
    v3: int32 = 5
    call test.main.Counter.bump(v2, v3): <'a>(ref<Counter, borrowed, 'a, exclusive, local>, int32) => void
    v4: Counter = local.get l0
    v5: int32 = field.get v4, 0
    return v5
}

/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_method_returning_this_to_its_owner_representation() {
    let session = TestSession::single(
        r#"
class User {
    id: int32 = 0;

    identity(): this {
        return this;
    }
}

function keep(user: User): User {
    return user.identity();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type User {
    id: int32;
}

function test.main.User.constructor(v0: ref<uninit<User>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<User>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.User.identity(v0: ref<User, managed, mutable, local>): ref<User, managed, mutable, local> {
entry(v0: ref<User, managed, mutable, local>):
    return v0
}

function test.main.keep(v0: ref<User, managed, mutable, local>): ref<User, managed, mutable, local> {
entry(v0: ref<User, managed, mutable, local>):
    v1: ref<User, managed, mutable, local> = call test.main.User.identity(v0): (ref<User, managed, mutable, local>) => ref<User, managed, mutable, local>
    return v1
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_enum_method_call_through_a_variant_receiver() {
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
@copy
type Status = variant<int64> { 1int64 = void; 2int64 = void; };

function test.main.Status.isActive<'a>(v0: ref<Status, borrowed, 'a, readonly, local>): boolean {
entry(v0: ref<Status, borrowed, 'a, readonly, local>):
    v1: Status = load v0
    v2: int64 = variant.tag v1
    v3: Status = variant.new 0
    v4: int64 = variant.tag v3
    v5: boolean = eq v2, v4
    return v5
}

function test.main.probe(v0: Status): boolean {
    local l0: Status

entry(v0: Status):
    local.set l0, v0
    v1: ref<Status, borrowed, 'frame, readonly, local> = local.address l0
    v2: boolean = call test.main.Status.isActive(v1): <'a>(ref<Status, borrowed, 'a, readonly, local>) => boolean
    return v2
}

/// @layout.variant name=Status size=8 align=8
/// @layout.discriminant owner=Status kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=Status index=0 discriminant=1 payload_offset=8
/// @layout.case owner=Status index=1 discriminant=2 payload_offset=8
"#,
    );
}

#[test]
fn test_lower_consuming_receiver_call_by_owned_value() {
    let session = TestSession::single(
        r#"
class Box {
    weight: int32 = 0;

    constructor(weight: int32) {
        this.weight = weight;
    }

    unwrap(^this): int32 {
        return this.weight;
    }
}

function open(): int32 {
    const parcel: ^Box = new Box(7);
    return parcel.unwrap();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Box {
    weight: int32;
}

function test.main.Box.constructor(v0: ref<uninit<Box>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<Box>, borrowed, exclusive, local>, v1: int32):
    v2: int32 = 0
    v3: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v3, v2
    v4: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v4, v1
    return
}

function test.main.Box.unwrap(v0: Box): int32 {
    local l0: Box

entry(v0: Box):
    local.set l0, v0
    v1: Box = local.get l0
    v2: int32 = field.get v1, 0
    return v2
}

function test.main.open(): int32 {
    local l0: Box

entry:
    v0: int32 = 7
    v1: ref<Box, borrowed, exclusive, frame> = local.address l0
    v2: ref<uninit<Box>, borrowed, exclusive, frame> = cast.bit v1 -> ref<uninit<Box>, borrowed, exclusive, frame>
    call test.main.Box.constructor(v2, v0): (ref<uninit<Box>, borrowed, exclusive, local>, int32) => void
    v3: Box = local.get l0
    v4: int32 = call test.main.Box.unwrap(v3): (Box) => int32
    return v4
}

/// @layout.struct name=Box size=4 align=4
/// @layout.field owner=Box index=0 name=weight offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_static_method_call_without_a_receiver() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;

    static origin(): int32 {
        return 0;
    }
}

function measure(): int32 {
    return Point.origin();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
}

function test.main.Point.origin(): int32 {
entry:
    v0: int32 = 0
    return v0
}

function test.main.measure(): int32 {
entry:
    v0: int32 = call test.main.Point.origin(): () => int32
    return v0
}

/// @layout.struct name=Point size=4 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_accessor_reads_and_writes_through_split_getter_and_setter() {
    let session = TestSession::single(
        r#"
struct Circle {
    radius: int32;

    get diameter(): int32 {
        return this.radius + this.radius;
    }

    set diameter(value: int32) {
        this.radius = value;
    }
}

function resize(): int32 {
    let circle = Circle { radius: 2 };
    circle.diameter = 10;
    return circle.diameter;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Circle {
    radius: int32;
}

function test.main.Circle.diameter.get<'a>(v0: ref<Circle, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<Circle, borrowed, 'a, readonly, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v4: int32 = load v3
    v5: int32 = add v2, v4
    return v5
}

function test.main.Circle.diameter.set<'a>(v0: ref<Circle, borrowed, 'a, exclusive, local>, v1: int32): void {
entry(v0: ref<Circle, borrowed, 'a, exclusive, local>, v1: int32):
    v2: ref<int32, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.resize(): int32 {
    local l0: Circle

entry:
    v0: int32 = 2
    v1: Circle = aggregate (v0)
    local.set l0, v1
    v2: ref<Circle, borrowed, 'frame, exclusive, local> = local.address l0
    v3: int32 = 10
    call test.main.Circle.diameter.set(v2, v3): <'a>(ref<Circle, borrowed, 'a, exclusive, local>, int32) => void
    v4: ref<Circle, borrowed, 'frame, readonly, local> = local.address l0
    v5: int32 = call test.main.Circle.diameter.get(v4): <'a>(ref<Circle, borrowed, 'a, readonly, local>) => int32
    return v5
}

/// @layout.struct name=Circle size=4 align=4
/// @layout.field owner=Circle index=0 name=radius offset=0 size=4 align=4
"#,
    );
}
