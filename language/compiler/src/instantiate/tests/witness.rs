use crate::tests::TestSession;

/// Dispatch a requirement call in a template through the receiver's witness at the instance.
#[test]
fn test_instantiate_dispatches_a_requirement_through_the_witness() {
    let session = TestSession::single(
        r#"
struct Path {
    steps: ^Array<int32>;
}

function duplicate<T: Clone>(value: &readonly T): T {
    return value.clone();
}

function main(): Path {
    const path = Path { steps: [] };
    return duplicate(path);
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.ds",
        "test.main.duplicate<test.main.Path>",
        r#"
@copy
type test.main.Path {
    steps: Array<int32>;
}

shared function test.main.duplicate<test.main.Path, 'a>(v0: ref<test.main.Path, borrowed, 'a, readonly, local>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Path, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Path, borrowed, 'a, readonly, local> = local.get l0
    v2: test.main.Path = call test.main.Clone.clone<test.main.Path>(v1): <'a>(ref<test.main.Path, borrowed, 'a, readonly, local>) => test.main.Path
    return v2
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
import { rc } from "destack:memory";

function duplicate<T: Clone>(value: &readonly T): T {
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
        "main.ds",
        "test.main.duplicate<Rc<int32>>",
        r#"
@languageItem("memory.rc.Rc")
type Rc<T>;

shared function test.main.duplicate<Rc<int32>, 'a>(v0: ref<Rc<int32>, borrowed, 'a, readonly, local>): Rc<int32> {
    local l0: ref<Rc<int32>, borrowed, 'a, readonly, local>

entry(v0: ref<Rc<int32>, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<Rc<int32>, borrowed, 'a, readonly, local> = local.get l0
    v2: Rc<int32> = call Rc.Clone.clone<int32>(v1): <'a>(ref<Rc<int32>, borrowed, 'a, readonly, local>) => Rc<int32>
    return v2
}
"#,
    );
    session.assert_mir_elaborated_function(
        "main.ds",
        "test.main.duplicate<Rc<int64>>",
        r#"
@languageItem("memory.rc.Rc")
type Rc<T>;

shared function test.main.duplicate<Rc<int64>, 'a>(v0: ref<Rc<int64>, borrowed, 'a, readonly, local>): Rc<int64> {
    local l0: ref<Rc<int64>, borrowed, 'a, readonly, local>

entry(v0: ref<Rc<int64>, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<Rc<int64>, borrowed, 'a, readonly, local> = local.get l0
    v2: Rc<int64> = call Rc.Clone.clone<int64>(v1): <'a>(ref<Rc<int64>, borrowed, 'a, readonly, local>) => Rc<int64>
    return v2
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
        "main.ds",
        "test.main.tagOf<test.main.Point>",
        r#"
@copy
type test.main.Point {
    x: int32;
}

shared function test.main.tagOf<test.main.Point>(): int32 {
entry:
    v1: ref<int32, borrowed, readonly, constant> = global.project test.main.Point.Tag
    v0: int32 = load v1
    return v0
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}
