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

    session.assert_mir_function(
        "main.tspp",
        "test.main.Point.length",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.Point.length<'a>(v0: ref<test.main.Point, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Point, borrowed, 'a, readonly>

entry(v0: ref<test.main.Point, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Point, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    v3: ref<test.main.Point, borrowed, 'a, readonly> = load l0
    v4: int32 = load (*v3).1
    v5: int32 = add v2, v4
    return v5
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.measure", r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.measure(): int32 {
    local l0: test.main.Point

entry:
    v0: int32 = 3
    v1: int32 = 4
    v2: test.main.Point = aggregate (v0, v1)
    store l0, v2
    v3: ref<test.main.Point, borrowed, 'frame, readonly> = address l0
    v4: int32 = call test.main.Point.length(v3): (ref<test.main.Point, borrowed, 'frame, readonly>) => int32
    return v4
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#);
}

#[test]
fn test_lower_struct_method_mutation_through_implicit_this() {
    let session = TestSession::single(
        r#"
struct Counter {
    count: int32;

    bump(&this, by: int32): void {
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

    session.assert_mir_function("main.tspp", "test.main.Counter.bump", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.bump<'a>(v0: ref<test.main.Counter, borrowed, 'a, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<test.main.Counter, borrowed, 'a, mutable>

entry(v0: ref<test.main.Counter, borrowed, 'a, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<test.main.Counter, borrowed, 'a, mutable> = load l1
    v3: int32 = load (*v2).0
    v4: int32 = load l0
    v5: int32 = add v3, v4
    store (*v2).0, v5
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.tally", r#"
type test.main.Counter {
    count: int32;
}

function test.main.tally(): int32 {
    local l0: test.main.Counter

entry:
    v0: int32 = 0
    v1: test.main.Counter = aggregate (v0)
    store l0, v1
    v2: int32 = 5
    v3: ref<test.main.Counter, borrowed, 'frame, mutable> = address l0
    call test.main.Counter.bump(v3, v2): (ref<test.main.Counter, borrowed, 'frame, mutable>, int32) => void
    v4: int32 = load (l0).0
    return v4
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.User.constructor",
        r#"
@nocopy
type test.main.User {
    id: int32;
}

constructor test.main.User.constructor<'a>(v0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.User>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.User.identity",
        r#"
@nocopy
type test.main.User {
    id: int32;
}

function test.main.User.identity(v0: ref<test.main.User, managed, mutable, local>): ref<test.main.User, managed, mutable, local> {
    local l0: ref<test.main.User, managed, mutable, local>

entry(v0: ref<test.main.User, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.User, managed, mutable, local> = load l0
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.keep", r#"
@nocopy
type test.main.User {
    id: int32;
}

function test.main.keep(v0: ref<test.main.User, managed, mutable, local>): ref<test.main.User, managed, mutable, local> {
    local l0: ref<test.main.User, managed, mutable, local>

entry(v0: ref<test.main.User, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.User, managed, mutable, local> = load l0
    v2: ref<test.main.User, managed, mutable, local> = call test.main.User.identity(v1): (ref<test.main.User, managed, mutable, local>) => ref<test.main.User, managed, mutable, local>
    return v2
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.Status.isActive",
        r#"
type test.main.Status = variant<uint8> { 1uint8 = void; 2uint8 = void; };

function test.main.Status.isActive<'a>(v0: ref<test.main.Status, borrowed, 'a, readonly>): boolean {
    local l0: ref<test.main.Status, borrowed, 'a, readonly>

entry(v0: ref<test.main.Status, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Status, borrowed, 'a, readonly> = load l0
    v2: test.main.Status = load (*v1)
    v3: uint8 = variant.tag v2
    v4: test.main.Status = variant.new 0
    v5: uint8 = variant.tag v4
    v6: boolean = eq v3, v5
    return v6
}

/// @layout.variant name=test.main.Status size=1 align=1
/// @layout.discriminant owner=test.main.Status kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Status index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Status index=1 discriminant=2 payload_offset=1
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.probe", r#"
type test.main.Status = variant<uint8> { 1uint8 = void; 2uint8 = void; };

function test.main.probe(v0: test.main.Status): boolean {
    local l0: test.main.Status

entry(v0: test.main.Status):
    store l0, v0
    v1: ref<test.main.Status, borrowed, 'frame, readonly> = address l0
    v2: boolean = call test.main.Status.isActive(v1): (ref<test.main.Status, borrowed, 'frame, readonly>) => boolean
    return v2
}

/// @layout.variant name=test.main.Status size=1 align=1
/// @layout.discriminant owner=test.main.Status kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Status index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Status index=1 discriminant=2 payload_offset=1
"#);
}

#[test]
fn test_lower_consuming_receiver_call_by_owned_value() {
    let session = TestSession::single(
        r#"
class Box {
    weight: int32 = 0;

    constructor(&exclusive this, weight: int32) {
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

    session.assert_mir_function("main.tspp", "test.main.Box.constructor", r#"
@nocopy
type test.main.Box {
    weight: int32;
}

constructor test.main.Box.constructor<'a>(v0: ref<uninit<test.main.Box>, borrowed, 'a, exclusive>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Box>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Box>, borrowed, 'a, exclusive>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Box>, borrowed, 'a, exclusive> = address (*l1)
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Box size=4 align=4
/// @layout.field owner=test.main.Box index=0 name=weight offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.Box.unwrap",
        r#"
@nocopy
type test.main.Box {
    weight: int32;
}

function test.main.Box.unwrap(v0: test.main.Box): int32 {
    local l0: test.main.Box

entry(v0: test.main.Box):
    store l0, v0
    v1: int32 = load (l0).0
    return v1
}

/// @layout.struct name=test.main.Box size=4 align=4
/// @layout.field owner=test.main.Box index=0 name=weight offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.open",
        r#"
@nocopy
type test.main.Box {
    weight: int32;
}

function test.main.open(): int32 {
    local l0: test.main.Box
    local l1: test.main.Box

entry:
    v0: int32 = 7
    v1: ref<test.main.Box, borrowed, 'frame, exclusive> = address l0
    v2: ref<uninit<test.main.Box>, borrowed, 'managed, exclusive> = cast.bit v1 -> ref<uninit<test.main.Box>, borrowed, 'managed, exclusive>
    call test.main.Box.constructor(v2, v0): <'a>(ref<uninit<test.main.Box>, borrowed, 'a, exclusive>, int32) => void
    v3: test.main.Box = load l0
    store l1, v3
    v4: test.main.Box = load l1
    v5: int32 = call test.main.Box.unwrap(v4): (test.main.Box) => int32
    return v5
}

/// @layout.struct name=test.main.Box size=4 align=4
/// @layout.field owner=test.main.Box index=0 name=weight offset=0 size=4 align=4
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.Point.origin",
        r#"
function test.main.Point.origin(): int32 {
entry:
    v0: int32 = 0
    return v0
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.measure",
        r#"
function test.main.measure(): int32 {
entry:
    v0: int32 = call test.main.Point.origin(): () => int32
    return v0
}
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.Circle.diameter.get",
        r#"
type test.main.Circle {
    radius: int32;
}

function test.main.Circle.diameter.get<'a>(v0: ref<test.main.Circle, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Circle, borrowed, 'a, readonly>

entry(v0: ref<test.main.Circle, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Circle, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    v3: ref<test.main.Circle, borrowed, 'a, readonly> = load l0
    v4: int32 = load (*v3).0
    v5: int32 = add v2, v4
    return v5
}

/// @layout.struct name=test.main.Circle size=4 align=4
/// @layout.field owner=test.main.Circle index=0 name=radius offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Circle.diameter.set", r#"
type test.main.Circle {
    radius: int32;
}

function test.main.Circle.diameter.set<'a>(v0: ref<test.main.Circle, borrowed, 'a, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<test.main.Circle, borrowed, 'a, mutable>

entry(v0: ref<test.main.Circle, borrowed, 'a, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<test.main.Circle, borrowed, 'a, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Circle size=4 align=4
/// @layout.field owner=test.main.Circle index=0 name=radius offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.resize", r#"
type test.main.Circle {
    radius: int32;
}

function test.main.resize(): int32 {
    local l0: test.main.Circle

entry:
    v0: int32 = 2
    v1: test.main.Circle = aggregate (v0)
    store l0, v1
    v2: int32 = 10
    v3: ref<test.main.Circle, borrowed, 'frame, mutable> = address l0
    call test.main.Circle.diameter.set(v3, v2): (ref<test.main.Circle, borrowed, 'frame, mutable>, int32) => void
    v4: ref<test.main.Circle, borrowed, 'frame, readonly> = address l0
    v5: int32 = call test.main.Circle.diameter.get(v4): (ref<test.main.Circle, borrowed, 'frame, readonly>) => int32
    return v5
}

/// @layout.struct name=test.main.Circle size=4 align=4
/// @layout.field owner=test.main.Circle index=0 name=radius offset=0 size=4 align=4
"#);
}

/// Recover a class handle from a borrow of a managed object by reinterpreting the borrow.
#[test]
fn test_lower_a_handle_read_from_a_managed_borrow() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32 = 0;

    peek(): int32 {
        return this.value;
    }
}

function read(counter: Counter): int32 {
    const view: &Counter = &counter;

    return view.peek();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
@nocopy
type test.main.Counter {
    value: int32;
}

function test.main.read(v0: ref<test.main.Counter, managed, mutable, local>): int32 {
    local l0: ref<test.main.Counter, managed, mutable, local>
    local l1: ref<test.main.Counter, borrowed, 'managed, mutable>

entry(v0: ref<test.main.Counter, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Counter, managed, mutable, local> = load l0
    v2: ref<test.main.Counter, borrowed, 'managed, mutable> = cast.bit v1 -> ref<test.main.Counter, borrowed, 'managed, mutable>
    store l1, v2
    v3: ref<test.main.Counter, borrowed, 'managed, mutable> = load l1
    v4: ref<test.main.Counter, managed, mutable, local> = cast.bit v3 -> ref<test.main.Counter, managed, mutable, local>
    v5: int32 = call test.main.Counter.peek(v4): (ref<test.main.Counter, managed, mutable, local>) => int32
    return v5
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=0 size=4 align=4
"#,
    );
}

/// Borrow a string object through its handle at the access the object grants.
#[test]
fn test_lower_a_string_borrow_through_its_handle() {
    let session = TestSession::single(
        r#"
function view(text: string): void {
    const slice = text.borrow();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.view",
        r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.view(v0: ref<String, managed, mutable, local>): void {
    local l0: ref<String, managed, mutable, local>
    local l1: slice<uint16, borrowed, 'managed, mutable>

entry(v0: ref<String, managed, mutable, local>):
    store l0, v0
    v1: ref<String, managed, mutable, local> = load l0
    v2: ref<String, borrowed, 'managed, mutable> = cast.bit v1 -> ref<String, borrowed, 'managed, mutable>
    v3: slice<uint16, borrowed, 'managed, mutable> = call String.Borrow.borrow<mutable>(v2): (ref<String, borrowed, 'managed, mutable>) => slice<uint16, borrowed, 'managed, mutable>
    store l1, v3
    return
}
"#,
    );
}
