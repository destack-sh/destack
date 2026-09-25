use crate::tests::TestSession;

/// Dispatch a requirement call in a template through the receiver's witness at the instance.
#[test]
fn test_instantiate_dispatches_a_requirement_through_the_witness() {
    let session = TestSession::single(
        r#"
struct Path {
    steps: ^Array<int32>;
}

function duplicate<T: Clone>(value: &immutable T): T {
    return value.clone();
}

function main(): Path {
    const path = Path { steps: [] };
    return duplicate(path);
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.duplicate<test.main.Path>",
        r#"
type test.main.Path {
    steps: Array<int32>;
}

shared function test.main.duplicate<test.main.Path, 'a>(v0: ref<test.main.Path, borrowed, 'a, immutable>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, immutable>

entry(v0: ref<test.main.Path, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Path, borrowed, 'a, immutable> = load l0
    v2: test.main.Path = call test.main.Clone.clone<test.main.Path>(v1): (ref<test.main.Path, borrowed, 'a, immutable>) => test.main.Path
    v3: test.main.Path = copy v2
    drop v2
    return v3
}

/// @layout.struct name=test.main.Path size=32 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
"#,
    );
}

/// Dispatch each closed receiver of one generic base through its own witness.
#[test]
fn test_instantiate_dispatches_each_receiver_of_one_base_through_its_own_witness() {
    let session = TestSession::single(
        r#"
import { rc } from "tspp:memory";

function duplicate<T: Clone>(value: &immutable T): T {
    return value.clone();
}

function main(): (rc.Rc<int32>, rc.Rc<int64>) {
    const first = rc.Rc.new(1 as int32);
    const second = rc.Rc.new(2 as int64);
    return (duplicate(first), duplicate(second));
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.duplicate<Rc<int32>>",
        r#"
@nocopy
@languageItem("memory.rc.Rc")
type Rc<T>;

shared function test.main.duplicate<Rc<int32>, 'a>(v0: ref<Rc<int32>, borrowed, 'a, immutable>): Rc<int32> {
    local l0: ref<Rc<int32>, borrowed, 'a, immutable>

entry(v0: ref<Rc<int32>, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<Rc<int32>, borrowed, 'a, immutable> = load l0
    v2: Rc<int32> = call Rc.Clone.clone<int32>(v1): (ref<Rc<int32>, borrowed, 'a, immutable>) => Rc<int32>
    v3: Rc<int32> = copy v2
    drop v2
    return v3
}
"#,
    );
    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.duplicate<Rc<int64>>",
        r#"
@nocopy
@languageItem("memory.rc.Rc")
type Rc<T>;

shared function test.main.duplicate<Rc<int64>, 'a>(v0: ref<Rc<int64>, borrowed, 'a, immutable>): Rc<int64> {
    local l0: ref<Rc<int64>, borrowed, 'a, immutable>

entry(v0: ref<Rc<int64>, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<Rc<int64>, borrowed, 'a, immutable> = load l0
    v2: Rc<int64> = call Rc.Clone.clone<int64>(v1): (ref<Rc<int64>, borrowed, 'a, immutable>) => Rc<int64>
    v3: Rc<int64> = copy v2
    drop v2
    return v3
}
"#,
    );
}

/// Instantiate an associated const read at a closed receiver as a load of the witness's global.
#[test]
fn test_instantiate_resolves_an_associated_const_through_the_witness() {
    let session = TestSession::single(
        r#"
interface Tagged {
    const Tag: int32;
}

struct Point implements Tagged {
    x: int32;
    const Tag: int32 = 7;
}

function tagOf<T: Tagged>(): int32 {
    return T.Tag;
}

function main(): int32 {
    return tagOf<Point>();
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.tagOf<test.main.Point>",
        r#"
type test.main.Point {
    x: int32;
}

shared function test.main.tagOf<test.main.Point>(): int32 {
entry:
    v0: int32 = load @test.main.Point.Tag
    return v0
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}

/// A specialization closing an erasure records the dynamic table of the closed payload.
#[test]
fn test_instantiate_the_dynamic_table_of_a_closed_erasure() {
    let session = TestSession::single(
        r#"
interface Greeter {
    greet(): int32;
}

class Console implements Greeter {
    greet(): int32 {
        return 1;
    }
}

function erase<T: Greeter>(value: T): Greeter {
    return value;
}

function run(): int32 {
    return erase(new Console()).greet();
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp", r#"
@nocopy
type test.main.Console { }

@nocopy
type test.main.Greeter { }

function test.main.Console.greet(v0: ref<test.main.Console, managed, mutable, local>): int32 {
    local l0: ref<test.main.Console, managed, mutable, local>

entry(v0: ref<test.main.Console, managed, mutable, local>):
    store l0, v0
    v1: int32 = 1
    return v1
}

function test.main.run(): int32 {
entry:
    v0: ref<test.main.Console, managed, mutable, local> = new.zeroed test.main.Console, local
    v1: dynamic<test.main.Greeter, managed, mutable, local> = call test.main.erase<ref<test.main.Console, managed, mutable, local>>(v0): (ref<test.main.Console, managed, mutable, local>) => dynamic<test.main.Greeter, managed, mutable, local>
    v2: int32 = call.dynamic v1, test.main.Greeter, 0(): () => int32
    return v2
}

function test.main.erase<T: test.main.Greeter>(v0: T): dynamic<test.main.Greeter, managed, mutable, local>;

external function test.main.Greeter.greet<this: test.main.Greeter>(this): int32

shared function test.main.erase<ref<test.main.Console, managed, mutable, local>>(v0: ref<test.main.Console, managed, mutable, local>): dynamic<test.main.Greeter, managed, mutable, local> {
    local l0: ref<test.main.Console, managed, mutable, local>

entry(v0: ref<test.main.Console, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Console, managed, mutable, local> = load l0
    v2: dynamic<test.main.Greeter, managed, mutable, local> = dynamic.bind v1, ref<test.main.Console, managed, mutable, local>
    return v2
}

/// @layout.struct name=test.main.Console size=0 align=1
/// @layout.struct name=test.main.Greeter size=0 align=1
/// @layout.struct name=type@2 size=0 align=1

/// @dispatch.shape constraint=type@5 function=greet
/// @dispatch.table concrete=type@1 constraint=type@5 function@1
"#,
    );
}
