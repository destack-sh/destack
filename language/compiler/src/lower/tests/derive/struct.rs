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

function same(left: &readonly Point, right: &readonly Point): boolean {
    return left.equal(right);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.same", r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.same<'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, readonly, local>, v1: ref<test.main.Point, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<test.main.Point, borrowed, 'a, readonly, local>
    local l1: ref<test.main.Point, borrowed, 'b, readonly, local>

entry(v0: ref<test.main.Point, borrowed, 'a, readonly, local>, v1: ref<test.main.Point, borrowed, 'b, readonly, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<test.main.Point, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<test.main.Point, borrowed, 'b, readonly, local> = local.get l1
    v4: boolean = call test.main.PartialEqual.equal<test.main.Point>(v2, v3): <'a, 'b>(ref<test.main.Point, borrowed, 'a, readonly, local>, ref<test.main.Point, borrowed, 'b, readonly, local>) => boolean
    return v4
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.PartialEqual.equal<test.main.Point>", r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.PartialEqual.equal<test.main.Point, 'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, readonly, local>, v1: ref<test.main.Point, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<test.main.Point, borrowed, 'b, readonly, local>
    local l1: ref<test.main.Point, borrowed, 'a, readonly, local>
    local l2: boolean

entry(v0: ref<test.main.Point, borrowed, 'a, readonly, local>, v1: ref<test.main.Point, borrowed, 'b, readonly, local>):
    local.set l0, v1
    local.set l1, v0
    v2: ref<test.main.Point, borrowed, 'a, readonly, local> = local.get l1
    v3: ref<test.main.Point, borrowed, 'b, readonly, local> = local.get l0
    v4: ref<int32, borrowed, 'b, readonly, local> = field.address v3, 0
    v5: ref<int32, borrowed, 'a, readonly, local> = field.address v2, 0
    v6: boolean = call Integer.PartialEqual.equal<int32>(v5, v4): <'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, ref<int32, borrowed, 'b, readonly, local>) => boolean
    local.set l2, v6
    branch v6 => b1 | b2

b1:
    v7: ref<test.main.Point, borrowed, 'a, readonly, local> = local.get l1
    v8: ref<test.main.Point, borrowed, 'b, readonly, local> = local.get l0
    v9: ref<int32, borrowed, 'b, readonly, local> = field.address v8, 1
    v10: ref<int32, borrowed, 'a, readonly, local> = field.address v7, 1
    v11: boolean = call Integer.PartialEqual.equal<int32>(v10, v9): <'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, ref<int32, borrowed, 'b, readonly, local>) => boolean
    local.set l2, v11
    jump b2

b2:
    v12: boolean = local.get l2
    return v12
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

function duplicate(path: &readonly Path): Path {
    return path.clone();
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.duplicate", r#"
@copy
type test.main.Path {
    steps: Array<int32>;
    weight: int32;
}

function test.main.duplicate<'a>(v0: ref<test.main.Path, borrowed, 'a, readonly, local>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Path, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Path, borrowed, 'a, readonly, local> = local.get l0
    v2: test.main.Path = call test.main.Clone.clone<test.main.Path>(v1): <'a>(ref<test.main.Path, borrowed, 'a, readonly, local>) => test.main.Path
    return v2
}

/// @layout.struct name=test.main.Path size=40 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
/// @layout.field owner=test.main.Path index=1 name=weight offset=32 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.Clone.clone<test.main.Path>", r#"
@languageItem("collections.Array")
type Array<T>;

@copy
type test.main.Path {
    steps: Array<int32>;
    weight: int32;
}

function test.main.Clone.clone<test.main.Path, 'a>(v0: ref<test.main.Path, borrowed, 'a, readonly, local>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Path, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Path, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<Array<int32>, borrowed, 'a, readonly, local> = field.address v1, 0
    v3: Array<int32> = call Array.Clone.clone<int32>(v2): <'a>(ref<Array<int32>, borrowed, 'a, readonly, local>) => Array<int32>
    v4: ref<test.main.Path, borrowed, 'a, readonly, local> = local.get l0
    v5: ref<int32, borrowed, 'a, readonly, local> = field.address v4, 1
    v6: int32 = call Integer.Clone.clone<int32>(v5): <'a>(ref<int32, borrowed, 'a, readonly, local>) => int32
    v7: test.main.Path = aggregate (v3, v6)
    return v7
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

function duplicate(point: &readonly Point): Point {
    return point.clone();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.duplicate",
        r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

@languageItem("memory.Clone")
type Clone;

function test.main.duplicate<'a>(v0: ref<test.main.Point, borrowed, 'a, readonly, local>): test.main.Point {
    local l0: ref<test.main.Point, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Point, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Point, borrowed, 'a, readonly, local> = local.get l0
    v2: test.main.Point = call.witness test.main.Point, Clone, Clone.clone(v1): <'a>(ref<test.main.Point, borrowed, 'a, readonly, local>) => test.main.Point
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
import { Hasher } from "destack:ops";

struct Point {
    x: int32;
    y: boolean;
}

function digest(point: &readonly Point, state: &Hasher): void {
    point.hash(state);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.digest", r#"
@copy
type test.main.Point {
    x: int32;
    y: boolean;
}

@languageItem("ops.Hasher")
type Hasher;

function test.main.digest<'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, readonly, local>, v1: ref<Hasher, borrowed, 'b, mutable, local>): void {
    local l0: ref<test.main.Point, borrowed, 'a, readonly, local>
    local l1: ref<Hasher, borrowed, 'b, mutable, local>

entry(v0: ref<test.main.Point, borrowed, 'a, readonly, local>, v1: ref<Hasher, borrowed, 'b, mutable, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<test.main.Point, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<Hasher, borrowed, 'b, mutable, local> = local.get l1
    v4: test.main.Point = load v2
    call test.main.Hash.hash<test.main.Point>(v4, v3): <'a>(test.main.Point, ref<Hasher, borrowed, 'a, mutable, local>) => void
    return
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=1 align=1
"#);

    session.assert_mir_function("main.ds", "test.main.Hash.hash<test.main.Point>", r#"
@copy
type test.main.Point {
    x: int32;
    y: boolean;
}

@languageItem("ops.Hasher")
type Hasher;

function test.main.Hash.hash<test.main.Point, 'a>(v0: test.main.Point, v1: ref<Hasher, borrowed, 'a, mutable, local>): void {
    local l0: ref<Hasher, borrowed, 'a, mutable, local>
    local l1: test.main.Point

entry(v0: test.main.Point, v1: ref<Hasher, borrowed, 'a, mutable, local>):
    local.set l0, v1
    local.set l1, v0
    v2: test.main.Point = local.get l1
    v3: int32 = field.get v2, 0
    v4: ref<Hasher, borrowed, 'a, mutable, local> = local.get l0
    call Integer.Hash.hash<int32>(v3, v4): <'a>(int32, ref<Hasher, borrowed, 'a, mutable, local>) => void
    v5: ref<Hasher, borrowed, 'a, mutable, local> = local.get l0
    v6: ref<test.main.Point, borrowed, 'frame, readonly, frame> = local.project l1
    v7: ref<boolean, borrowed, 'frame, readonly, local> = field.address v6, 1
    call Boolean.Hash.hash(v7, v5): <'a, 'b>(ref<boolean, borrowed, 'a, readonly, local>, ref<Hasher, borrowed, 'b, mutable, local>) => void
    return
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=1 align=1
"#);
}
