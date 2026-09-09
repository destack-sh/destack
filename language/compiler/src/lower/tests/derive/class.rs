use crate::tests::TestSession;

/// Lower a derived class equality comparing managed identities.
#[test]
fn test_lower_a_derived_class_equality() {
    let session = TestSession::single(
        r#"
import { PartialEqual } from "destack:ops";

class Node {
    value: int32 = 0;
}

function same<T: PartialEqual<T>>(left: &readonly T, right: &readonly T): boolean {
    return left.equal(right);
}

function compare(left: &readonly Node, right: &readonly Node): boolean {
    return same(left, right);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Node.constructor",
        r#"
type test.main.Node {
    value: int32;
}

function test.main.Node.constructor<'a>(v0: ref<uninit<test.main.Node>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Node>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Node>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Node>, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 0
    v3: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
    return
}

/// @layout.struct name=test.main.Node size=4 align=4
/// @layout.field owner=test.main.Node index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.compare", r#"
type test.main.Node {
    value: int32;
}

function test.main.compare<'a, 'b>(v0: ref<test.main.Node, borrowed, 'a, readonly, local>, v1: ref<test.main.Node, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<test.main.Node, borrowed, 'a, readonly, local>
    local l1: ref<test.main.Node, borrowed, 'b, readonly, local>

entry(v0: ref<test.main.Node, borrowed, 'a, readonly, local>, v1: ref<test.main.Node, borrowed, 'b, readonly, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<test.main.Node, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<test.main.Node, borrowed, 'b, readonly, local> = local.get l1
    v4: boolean = call test.main.same<ref<test.main.Node, managed, mutable, local>>(v2, v3): <'a, 'b>(ref<ref<test.main.Node, managed, mutable, local>, borrowed, 'a, readonly, local>, ref<ref<test.main.Node, managed, mutable, local>, borrowed, 'b, readonly, local>) => boolean
    return v4
}

/// @layout.struct name=test.main.Node size=4 align=4
/// @layout.field owner=test.main.Node index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.same<ref<test.main.Node, managed, mutable, local>>", r#"
type test.main.Node {
    value: int32;
}

shared function test.main.same<ref<test.main.Node, managed, mutable, local>, 'a, 'b>(v0: ref<ref<test.main.Node, managed, mutable, local>, borrowed, 'a, readonly, local>, v1: ref<ref<test.main.Node, managed, mutable, local>, borrowed, 'b, readonly, local>): boolean;

/// @layout.struct name=test.main.Node size=4 align=4
/// @layout.field owner=test.main.Node index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.PartialEqual.equal<ref<test.main.Node, managed, mutable, local>>",
        r#"
type test.main.Node {
    value: int32;
}

function test.main.PartialEqual.equal<ref<test.main.Node, managed, mutable, local>, 'a, 'b>(v0: ref<test.main.Node, borrowed, 'a, readonly, local>, v1: ref<test.main.Node, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<test.main.Node, borrowed, 'b, readonly, local>
    local l1: ref<test.main.Node, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Node, borrowed, 'a, readonly, local>, v1: ref<test.main.Node, borrowed, 'b, readonly, local>):
    local.set l0, v1
    local.set l1, v0
    v2: ref<test.main.Node, borrowed, 'a, readonly, local> = local.get l1
    v3: ref<test.main.Node, managed, mutable, local> = load v2
    v4: ref<test.main.Node, borrowed, 'b, readonly, local> = local.get l0
    v5: ref<test.main.Node, managed, mutable, local> = load v4
    v6: boolean = eq v3, v5
    return v6
}

/// @layout.struct name=test.main.Node size=4 align=4
/// @layout.field owner=test.main.Node index=0 name=value offset=0 size=4 align=4
"#,
    );
}

/// Lower a derived class clone allocating a fresh object with the fields along its heritage
/// cloned.
#[test]
fn test_lower_a_derived_class_clone() {
    let session = TestSession::single(
        r#"
class Base {
    id: int32 = 1;
}

class Node extends Base {
    value: int32 = 0;
}

function duplicate<T: Clone>(value: &readonly T): T {
    return value.clone();
}

function copy(value: &readonly Node): Node {
    return duplicate(value);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.copy", r#"
type test.main.Node {
    id: int32;
    value: int32;
}

function test.main.copy<'a>(v0: ref<test.main.Node, borrowed, 'a, readonly, local>): ref<test.main.Node, managed, mutable, local> {
    local l0: ref<test.main.Node, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Node, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Node, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<test.main.Node, managed, mutable, local> = call test.main.duplicate<ref<test.main.Node, managed, mutable, local>>(v1): <'a>(ref<ref<test.main.Node, managed, mutable, local>, borrowed, 'a, readonly, local>) => ref<test.main.Node, managed, mutable, local>
    return v2
}

/// @layout.struct name=test.main.Node size=8 align=4
/// @layout.field owner=test.main.Node index=0 name=id offset=0 size=4 align=4
/// @layout.field owner=test.main.Node index=1 name=value offset=4 size=4 align=4
"#);
    session.assert_mir_function(
        "main.ds",
        "test.main.Clone.clone<ref<test.main.Node, managed, mutable, local>>",
        r#"
type test.main.Node {
    id: int32;
    value: int32;
}

function test.main.Clone.clone<ref<test.main.Node, managed, mutable, local>, 'a>(v0: ref<test.main.Node, borrowed, 'a, readonly, local>): test.main.Node {
    local l0: ref<test.main.Node, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Node, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Node, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<int32, borrowed, 'a, readonly, local> = field.address v1, 0
    v3: int32 = call Integer.Clone.clone<int32>(v2): <'a>(ref<int32, borrowed, 'a, readonly, local>) => int32
    v4: ref<test.main.Node, borrowed, 'a, readonly, local> = local.get l0
    v5: ref<int32, borrowed, 'a, readonly, local> = field.address v4, 1
    v6: int32 = call Integer.Clone.clone<int32>(v5): <'a>(ref<int32, borrowed, 'a, readonly, local>) => int32
    v7: test.main.Node = aggregate (v3, v6)
    return v7
}

/// @layout.struct name=test.main.Node size=8 align=4
/// @layout.field owner=test.main.Node index=0 name=id offset=0 size=4 align=4
/// @layout.field owner=test.main.Node index=1 name=value offset=4 size=4 align=4
"#,
    );
}
