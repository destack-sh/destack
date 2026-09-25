use crate::tests::TestSession;

/// Lower a derived class equality comparing fields.
#[test]
fn test_lower_a_derived_class_equality() {
    let session = TestSession::single(
        r#"
import { PartialEqual } from "tspp:ops";

class Node {
    value: int32 = 0;
}

function same<T: PartialEqual<T>>(left: &immutable T, right: &immutable T): boolean {
    return left.equal(right);
}

function compare(left: &immutable Node, right: &immutable Node): boolean {
    return same(left, right);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Node.constructor",
        r#"
@nocopy
type test.main.Node {
    value: int32;
}

constructor test.main.Node.constructor<'a>(v0: ref<uninit<test.main.Node>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Node>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Node>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Node>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Node size=4 align=4
/// @layout.field owner=test.main.Node index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.compare", r#"
@nocopy
type test.main.Node {
    value: int32;
}

function test.main.compare<'a, 'b>(v0: ref<test.main.Node, borrowed, 'a, immutable>, v1: ref<test.main.Node, borrowed, 'b, immutable>): boolean {
    local l0: ref<test.main.Node, borrowed, 'a, immutable>
    local l1: ref<test.main.Node, borrowed, 'b, immutable>

entry(v0: ref<test.main.Node, borrowed, 'a, immutable>, v1: ref<test.main.Node, borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<test.main.Node, borrowed, 'a, immutable> = load l0
    v3: ref<test.main.Node, borrowed, 'b, immutable> = load l1
    v4: boolean = call test.main.same<ref<test.main.Node, managed, mutable, local>>(v2, v3): (ref<test.main.Node, borrowed, 'a, immutable>, ref<test.main.Node, borrowed, 'b, immutable>) => boolean
    return v4
}

/// @layout.struct name=test.main.Node size=4 align=4
/// @layout.field owner=test.main.Node index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.same", r#"
@nocopy
@languageItem("ops.PartialEqual")
type PartialEqual<T>;

function test.main.same<T: PartialEqual<T>, 'a, 'b>(v0: ref<?T, borrowed, 'a, immutable>, v1: ref<?T, borrowed, 'b, immutable>): boolean {
    local l0: ref<?T, borrowed, 'a, immutable>
    local l1: ref<?T, borrowed, 'b, immutable>

entry(v0: ref<?T, borrowed, 'a, immutable>, v1: ref<?T, borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<?T, borrowed, 'a, immutable> = load l0
    v3: ref<?T, borrowed, 'b, immutable> = load l1
    v4: boolean = call.witness T, PartialEqual<T>, PartialEqual.equal(v2, v3): (ref<?T, borrowed, 'a, immutable>, ref<?T, borrowed, 'b, immutable>) => boolean
    return v4
}
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.PartialEqual.equal<ref<test.main.Node, managed, mutable, local>>",
        r#"
@nocopy
type test.main.Node {
    value: int32;
}

function test.main.PartialEqual.equal<ref<test.main.Node, managed, mutable, local>, 'a, 'b>(v0: ref<test.main.Node, borrowed, 'a, immutable>, v1: ref<test.main.Node, borrowed, 'b, immutable>): boolean {
    local l0: ref<test.main.Node, borrowed, 'b, immutable>
    local l1: ref<test.main.Node, borrowed, 'a, immutable>

entry(v0: ref<test.main.Node, borrowed, 'a, immutable>, v1: ref<test.main.Node, borrowed, 'b, immutable>):
    store l0, v1
    store l1, v0
    v2: ref<test.main.Node, borrowed, 'a, immutable> = load l1
    v3: ref<test.main.Node, borrowed, 'b, immutable> = load l0
    v4: ref<int32, borrowed, 'b, immutable> = address (*v3).0
    v5: ref<int32, borrowed, 'a, immutable> = address (*v2).0
    v6: boolean = call Integer.PartialEqual.equal<int32>(v5, v4): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
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

function duplicate<T: Clone>(value: &immutable T): ^T {
    return value.clone();
}

function copy(value: &immutable Node): ^Node {
    return duplicate(value);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.copy", r#"
@nocopy
type test.main.Node {
    id: int32;
    value: int32;
}

function test.main.copy<'a>(v0: ref<test.main.Node, borrowed, 'a, immutable>): test.main.Node {
    local l0: ref<test.main.Node, borrowed, 'a, immutable>

entry(v0: ref<test.main.Node, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Node, borrowed, 'a, immutable> = load l0
    v2: test.main.Node = call test.main.duplicate<ref<test.main.Node, managed, mutable, local>>(v1): (ref<test.main.Node, borrowed, 'a, immutable>) => test.main.Node
    return v2
}

/// @layout.struct name=test.main.Node size=8 align=4
/// @layout.field owner=test.main.Node index=0 name=id offset=0 size=4 align=4
/// @layout.field owner=test.main.Node index=1 name=value offset=4 size=4 align=4
"#);
    session.assert_mir_function(
        "main.tspp",
        "test.main.Clone.clone<ref<test.main.Node, managed, mutable, local>>",
        r#"
@nocopy
type test.main.Node {
    id: int32;
    value: int32;
}

function test.main.Clone.clone<ref<test.main.Node, managed, mutable, local>, 'a>(v0: ref<test.main.Node, borrowed, 'a, immutable>): test.main.Node {
    local l0: ref<test.main.Node, borrowed, 'a, immutable>

entry(v0: ref<test.main.Node, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Node, borrowed, 'a, immutable> = load l0
    v2: ref<int32, borrowed, 'a, immutable> = address (*v1).0
    v3: int32 = call Integer.Clone.clone<int32>(v2): (ref<int32, borrowed, 'a, immutable>) => int32
    v4: ref<test.main.Node, borrowed, 'a, immutable> = load l0
    v5: ref<int32, borrowed, 'a, immutable> = address (*v4).1
    v6: int32 = call Integer.Clone.clone<int32>(v5): (ref<int32, borrowed, 'a, immutable>) => int32
    v7: test.main.Node = aggregate (v3, v6)
    return v7
}

/// @layout.struct name=test.main.Node size=8 align=4
/// @layout.field owner=test.main.Node index=0 name=id offset=0 size=4 align=4
/// @layout.field owner=test.main.Node index=1 name=value offset=4 size=4 align=4
"#,
    );
}

/// Lower a derived class hash feeding each field to the hasher.
#[test]
fn test_lower_a_derived_class_hash_field_wise() {
    let session = TestSession::single(
        r#"
import { Hasher } from "tspp:ops";

class Point {
    x: int32 = 0;
    y: boolean = false;
}

function digest(point: &immutable Point, state: &Hasher): void {
    point.hash(state);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.digest", r#"
@nocopy
type test.main.Point {
    x: int32;
    y: boolean;
}

@nocopy
@languageItem("ops.Hasher")
type Hasher;

@nocopy
@languageItem("ops.Hash")
type Hash;

function test.main.digest<'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>): void {
    local l0: ref<test.main.Point, borrowed, 'a, immutable>
    local l1: dynamic<Hasher, borrowed, 'b, mutable>

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>):
    store l0, v0
    store l1, v1
    v2: ref<test.main.Point, borrowed, 'a, immutable> = load l0
    v3: dynamic<Hasher, borrowed, 'b, mutable> = load l1
    call.witness ref<test.main.Point, managed, mutable, local>, Hash, Hash.hash(v2, v3): (ref<test.main.Point, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    return
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=1 align=1
"#);
}
