use crate::tests::TestSession;

/// Lower a derived struct equality to a field-wise comparison over the synthesized body.
#[test]
fn test_lower_a_derived_struct_equality_field_wise() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function same(left: &immutable Point, right: &immutable Point): boolean {
    return left.equal(right);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.same", r#"
type test.main.Point {
    x: int32;
    y: int32;
}

@nocopy
@languageItem("ops.PartialEqual")
type PartialEqual<T>;

export function test.main.same<'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: ref<test.main.Point, borrowed, 'b, immutable>): boolean {
    local l0: ref<test.main.Point, borrowed, 'a, immutable>
    local l1: ref<test.main.Point, borrowed, 'b, immutable>

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: ref<test.main.Point, borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<test.main.Point, borrowed, 'a, immutable> = load l0
    v3: ref<test.main.Point, borrowed, 'b, immutable> = load l1
    v4: boolean = call.witness test.main.Point, PartialEqual<test.main.Point>, PartialEqual.equal(v2, v3): (ref<test.main.Point, borrowed, 'a, immutable>, ref<test.main.Point, borrowed, 'b, immutable>) => boolean
    return v4
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.PartialEqual.equal<test.main.Point>", r#"
type test.main.Point {
    x: int32;
    y: int32;
}

export function test.main.PartialEqual.equal<test.main.Point, 'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: ref<test.main.Point, borrowed, 'b, immutable>): boolean {
    local l0: ref<test.main.Point, borrowed, 'b, immutable>
    local l1: ref<test.main.Point, borrowed, 'a, immutable>
    local l2: boolean

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: ref<test.main.Point, borrowed, 'b, immutable>):
    store l0, v1
    store l1, v0
    v2: ref<int32, borrowed, 'b, immutable> = address (*l0).0
    v3: ref<int32, borrowed, 'a, immutable> = address (*l1).0
    v4: boolean = call Integer.PartialEqual.equal<int32>(v3, v2): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
    store l2, v4
    branch v4 => b1 | b2

b1:
    v5: ref<int32, borrowed, 'b, immutable> = address (*l0).1
    v6: ref<int32, borrowed, 'a, immutable> = address (*l1).1
    v7: boolean = call Integer.PartialEqual.equal<int32>(v6, v5): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
    store l2, v7
    jump b2

b2:
    v8: boolean = load l2
    return v8
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#);
}

/// Lower a derived struct clone to a struct literal over every field's clone.
#[test]
fn test_lower_a_derived_struct_clone_field_wise() {
    let session = TestSession::single(
        r#"
struct Path {
    steps: ^Array<int32>;
    weight: int32;
}

function duplicate(path: &immutable Path): Path {
    return path.clone();
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.duplicate", r#"
type test.main.Path {
    steps: Array<int32>;
    weight: int32;
}

@nocopy
@languageItem("memory.Clone")
type Clone;

export function test.main.duplicate<'a>(v0: ref<test.main.Path, borrowed, 'a, immutable>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, immutable>

entry(v0: ref<test.main.Path, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Path, borrowed, 'a, immutable> = load l0
    v2: test.main.Path = call.witness test.main.Path, Clone, Clone.clone(v1): (ref<test.main.Path, borrowed, 'a, immutable>) => test.main.Path
    return v2
}

/// @layout.struct name=test.main.Path size=40 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
/// @layout.field owner=test.main.Path index=1 name=weight offset=32 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.Clone.clone<test.main.Path>", r#"
type test.main.Path {
    steps: Array<int32>;
    weight: int32;
}

@nocopy
@languageItem("collections.Array")
type Array<T>;

export function test.main.Clone.clone<test.main.Path, 'a>(v0: ref<test.main.Path, borrowed, 'a, immutable>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, immutable>

entry(v0: ref<test.main.Path, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<Array<int32>, borrowed, 'a, immutable> = address (*l0).0
    v2: Array<int32> = call Array.Clone.clone<int32>(v1): (ref<Array<int32>, borrowed, 'a, immutable>) => Array<int32>
    v3: ref<int32, borrowed, 'a, immutable> = address (*l0).1
    v4: int32 = call Integer.Clone.clone<int32>(v3): (ref<int32, borrowed, 'a, immutable>) => int32
    v5: test.main.Path = aggregate (v2, v4)
    return v5
}

/// @layout.struct name=test.main.Path size=40 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
/// @layout.field owner=test.main.Path index=1 name=weight offset=32 size=4 align=4
"#);
}

/// Lower a clone of a bitwise-copy struct to a load through its receiver.
#[test]
fn test_lower_a_clone_of_a_copy_struct_as_a_load() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function duplicate(point: &immutable Point): Point {
    return point.clone();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.duplicate",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

@nocopy
@languageItem("memory.Clone")
type Clone;

export function test.main.duplicate<'a>(v0: ref<test.main.Point, borrowed, 'a, immutable>): test.main.Point {
    local l0: ref<test.main.Point, borrowed, 'a, immutable>

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Point, borrowed, 'a, immutable> = load l0
    v2: test.main.Point = call.witness test.main.Point, Clone, Clone.clone(v1): (ref<test.main.Point, borrowed, 'a, immutable>) => test.main.Point
    return v2
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

/// Lower a derived struct hash to one hash call per field into the state.
#[test]
fn test_lower_a_derived_struct_hash_field_wise() {
    let session = TestSession::single(
        r#"
import { Hasher } from "tspp:ops";

struct Point {
    x: int32;
    y: boolean;
}

function digest(point: &immutable Point, state: &Hasher): void {
    point.hash(state);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.digest", r#"
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

export function test.main.digest<'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>): void {
    local l0: ref<test.main.Point, borrowed, 'a, immutable>
    local l1: dynamic<Hasher, borrowed, 'b, mutable>

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>):
    store l0, v0
    store l1, v1
    v2: ref<test.main.Point, borrowed, 'a, immutable> = load l0
    v3: dynamic<Hasher, borrowed, 'b, mutable> = load l1
    call.witness test.main.Point, Hash, Hash.hash(v2, v3): (ref<test.main.Point, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    return
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=1 align=1
"#);

    session.assert_mir_function("main.tspp", "test.main.Hash.hash<test.main.Point>", r#"
type test.main.Point {
    x: int32;
    y: boolean;
}

@nocopy
@languageItem("ops.Hasher")
type Hasher;

export function test.main.Hash.hash<test.main.Point, 'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>): void {
    local l0: dynamic<Hasher, borrowed, 'b, mutable>
    local l1: ref<test.main.Point, borrowed, 'a, immutable>

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>):
    store l0, v1
    store l1, v0
    v2: dynamic<Hasher, borrowed, 'b, mutable> = load l0
    v3: ref<int32, borrowed, 'a, immutable> = address (*l1).0
    call Integer.Hash.hash<int32>(v3, v2): (ref<int32, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    v4: dynamic<Hasher, borrowed, 'b, mutable> = load l0
    v5: ref<boolean, borrowed, 'a, immutable> = address (*l1).1
    call Boolean.Hash.hash(v5, v4): (ref<boolean, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    return
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=1 align=1
"#);
}
