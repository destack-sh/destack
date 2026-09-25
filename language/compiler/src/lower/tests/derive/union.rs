use crate::tests::TestSession;

/// Lower a derived union equality dispatching on both members.
#[test]
fn test_lower_a_derived_union_equality() {
    let session = TestSession::single(
        r#"
import { PartialEqual } from "tspp:ops";

struct Point {
    x: int32;
    y: int32;
}

function same<T: PartialEqual<T>>(left: &immutable T, right: &immutable T): boolean {
    return left.equal(right);
}

function compare(left: &immutable (Point | int32), right: &immutable (Point | int32)): boolean {
    return same(left, right);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.compare", r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.compare<'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable>): boolean {
    local l0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable>
    local l1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable>

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable> = load l0
    v3: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable> = load l1
    v4: boolean = call test.main.same<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }>(v2, v3): (ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable>, ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable>) => boolean
    return v4
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@4 size=12 align=4
/// @layout.discriminant owner=type@4 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@4 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@4 index=1 discriminant=1 payload_offset=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.same",
        r#"
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
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.PartialEqual.equal<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }>",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

@nocopy
@languageItem("ops.PartialEqual")
type PartialEqual<T>;

function test.main.PartialEqual.equal<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, 'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable>): boolean {
    local l0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable>
    local l1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable>
    local l2: boolean
    local l3: boolean, readonly
    local l4: boolean, readonly
    local l5: boolean, readonly

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable>):
    store l0, v1
    store l1, v0
    v2: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable> = load l1
    v3: uint1 = variant.tag.load (*v2)
    switch v3, b1, 0 => b3

b1:
    v5: boolean = false
    store l3, v5
    jump b2

b2:
    v6: boolean = load l3
    branch v6 => b4 | b5

b3:
    v4: boolean = true
    store l3, v4
    jump b2

b4:
    v7: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable> = load l0
    v8: uint1 = variant.tag.load (*v7)
    switch v8, b7, 0 => b9

b5:
    v20: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable> = load l0
    v21: uint1 = variant.tag.load (*v20)
    switch v21, b13, 1 => b15

b6:
    v33: boolean = load l2
    return v33

b7:
    v10: boolean = false
    store l4, v10
    jump b8

b8:
    v11: boolean = load l4
    branch v11 => b10 | b11

b9:
    v9: boolean = true
    store l4, v9
    jump b8

b10:
    v12: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable> = load l1
    v13: ref<test.main.Point, borrowed, 'a, immutable> = address ((*v12) as 0)
    v14: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable> = load l0
    v15: ref<test.main.Point, borrowed, 'b, immutable> = address ((*v14) as 0)
    v16: ref<test.main.Point, borrowed, 'b, immutable> = address (*v15)
    v17: ref<test.main.Point, borrowed, 'a, immutable> = address (*v13)
    v18: boolean = call.witness test.main.Point, PartialEqual<test.main.Point>, PartialEqual.equal(v17, v16): (ref<test.main.Point, borrowed, 'a, immutable>, ref<test.main.Point, borrowed, 'b, immutable>) => boolean
    store l2, v18
    jump b12

b11:
    v19: boolean = false
    store l2, v19
    jump b12

b12:
    jump b6

b13:
    v23: boolean = false
    store l5, v23
    jump b14

b14:
    v24: boolean = load l5
    branch v24 => b16 | b17

b15:
    v22: boolean = true
    store l5, v22
    jump b14

b16:
    v25: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, immutable> = load l1
    v26: ref<int32, borrowed, 'a, immutable> = address ((*v25) as 1)
    v27: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, immutable> = load l0
    v28: ref<int32, borrowed, 'b, immutable> = address ((*v27) as 1)
    v29: ref<int32, borrowed, 'b, immutable> = address (*v28)
    v30: ref<int32, borrowed, 'a, immutable> = address (*v26)
    v31: boolean = call Integer.PartialEqual.equal<int32>(v30, v29): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
    store l2, v31
    jump b18

b17:
    v32: boolean = false
    store l2, v32
    jump b18

b18:
    jump b6
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@4 size=12 align=4
/// @layout.discriminant owner=type@4 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@4 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@4 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.PartialEqual.equal<test.main.Point>", r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.PartialEqual.equal<test.main.Point, 'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: ref<test.main.Point, borrowed, 'b, immutable>): boolean {
    local l0: ref<test.main.Point, borrowed, 'b, immutable>
    local l1: ref<test.main.Point, borrowed, 'a, immutable>
    local l2: boolean

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: ref<test.main.Point, borrowed, 'b, immutable>):
    store l0, v1
    store l1, v0
    v2: ref<test.main.Point, borrowed, 'a, immutable> = load l1
    v3: ref<test.main.Point, borrowed, 'b, immutable> = load l0
    v4: ref<int32, borrowed, 'b, immutable> = address (*v3).0
    v5: ref<int32, borrowed, 'a, immutable> = address (*v2).0
    v6: boolean = call Integer.PartialEqual.equal<int32>(v5, v4): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
    store l2, v6
    branch v6 => b1 | b2

b1:
    v7: ref<test.main.Point, borrowed, 'a, immutable> = load l1
    v8: ref<test.main.Point, borrowed, 'b, immutable> = load l0
    v9: ref<int32, borrowed, 'b, immutable> = address (*v8).1
    v10: ref<int32, borrowed, 'a, immutable> = address (*v7).1
    v11: boolean = call Integer.PartialEqual.equal<int32>(v10, v9): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
    store l2, v11
    jump b2

b2:
    v12: boolean = load l2
    return v12
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#);
}

/// Lower a derived union hash writing the member position before its value.
#[test]
fn test_lower_a_derived_union_hash() {
    let session = TestSession::single(
        r#"
import { Hash, Hasher } from "tspp:ops";

struct Point {
    x: int32;
    y: int32;
}

function digest<T: Hash>(value: &immutable T, state: &Hasher): void {
    value.hash(state);
}

function feed(value: &immutable (Point | null), state: &Hasher): void {
    digest(value, state);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.feed", r#"
type test.main.Point {
    x: int32;
    y: int32;
}

@nocopy
@languageItem("ops.Hasher")
type Hasher;

function test.main.feed<'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>): void {
    local l0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable>
    local l1: dynamic<Hasher, borrowed, 'b, mutable>

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>):
    store l0, v0
    store l1, v1
    v2: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable> = load l0
    v3: dynamic<Hasher, borrowed, 'b, mutable> = load l1
    call test.main.digest<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }>(v2, v3): (ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    return
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@5 size=12 align=4
/// @layout.discriminant owner=type@5 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@5 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@5 index=1 discriminant=1 payload_offset=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.digest",
        r#"
@nocopy
@languageItem("ops.Hasher")
type Hasher;

@nocopy
@languageItem("ops.Hash")
type Hash;

function test.main.digest<T: Hash, 'a, 'b>(v0: ref<?T, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>): void {
    local l0: ref<?T, borrowed, 'a, immutable>
    local l1: dynamic<Hasher, borrowed, 'b, mutable>

entry(v0: ref<?T, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>):
    store l0, v0
    store l1, v1
    v2: ref<?T, borrowed, 'a, immutable> = load l0
    v3: dynamic<Hasher, borrowed, 'b, mutable> = load l1
    call.witness T, Hash, Hash.hash(v2, v3): (ref<?T, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    return
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Hash.hash<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }>",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

@nocopy
@languageItem("ops.Hasher")
type Hasher;

@nocopy
@languageItem("ops.Hash")
type Hash;

function test.main.Hash.hash<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, 'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>): void {
    local l0: dynamic<Hasher, borrowed, 'b, mutable>
    local l1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable>
    local l2: boolean, readonly
    local l3: usize
    local l4: usize

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>):
    store l0, v1
    store l1, v0
    v2: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable> = load l1
    v3: uint1 = variant.tag.load (*v2)
    switch v3, b1, 0 => b3

b1:
    v5: boolean = false
    store l2, v5
    jump b2

b2:
    v6: boolean = load l2
    branch v6 => b4 | b6

b3:
    v4: boolean = true
    store l2, v4
    jump b2

b4:
    v7: usize = 0
    store l3, v7
    v8: dynamic<Hasher, borrowed, 'b, mutable> = load l0
    v9: ref<usize, borrowed, 'frame, immutable> = address l3
    call Integer.Hash.hash<usize>(v9, v8): (ref<usize, borrowed, 'frame, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    v10: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, immutable> = load l1
    v11: ref<test.main.Point, borrowed, 'a, immutable> = address ((*v10) as 0)
    v12: dynamic<Hasher, borrowed, 'b, mutable> = load l0
    v13: ref<test.main.Point, borrowed, 'a, immutable> = address (*v11)
    call.witness test.main.Point, Hash, Hash.hash(v13, v12): (ref<test.main.Point, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    jump b5

b5:
    return

b6:
    v14: usize = 1
    store l4, v14
    v15: dynamic<Hasher, borrowed, 'b, mutable> = load l0
    v16: ref<usize, borrowed, 'frame, immutable> = address l4
    call Integer.Hash.hash<usize>(v16, v15): (ref<usize, borrowed, 'frame, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    jump b5
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@5 size=12 align=4
/// @layout.discriminant owner=type@5 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@5 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@5 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Hash.hash<test.main.Point>", r#"
type test.main.Point {
    x: int32;
    y: int32;
}

@nocopy
@languageItem("ops.Hasher")
type Hasher;

function test.main.Hash.hash<test.main.Point, 'a, 'b>(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>): void {
    local l0: dynamic<Hasher, borrowed, 'b, mutable>
    local l1: ref<test.main.Point, borrowed, 'a, immutable>

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>, v1: dynamic<Hasher, borrowed, 'b, mutable>):
    store l0, v1
    store l1, v0
    v2: ref<test.main.Point, borrowed, 'a, immutable> = load l1
    v3: dynamic<Hasher, borrowed, 'b, mutable> = load l0
    v4: ref<int32, borrowed, 'a, immutable> = address (*v2).0
    call Integer.Hash.hash<int32>(v4, v3): (ref<int32, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    v5: ref<test.main.Point, borrowed, 'a, immutable> = load l1
    v6: dynamic<Hasher, borrowed, 'b, mutable> = load l0
    v7: ref<int32, borrowed, 'a, immutable> = address (*v5).1
    call Integer.Hash.hash<int32>(v7, v6): (ref<int32, borrowed, 'a, immutable>, dynamic<Hasher, borrowed, 'b, mutable>) => void
    return
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#);
}

/// Lower a derived union clone entering the union again at the cloned member.
#[test]
fn test_lower_a_derived_union_clone() {
    let session = TestSession::single(
        r#"
struct Path {
    steps: ^Array<int32>;
}

function duplicate<T: Clone>(value: &immutable T): T {
    return value.clone();
}

function copy(value: &immutable (Path | int32)): Path | int32 {
    return duplicate(value);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.copy", r#"
type test.main.Path {
    steps: Array<int32>;
}

function test.main.copy<'a>(v0: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable>): variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; } {
    local l0: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable>

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable> = load l0
    v2: variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; } = call test.main.duplicate<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }>(v1): (ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable>) => variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }
    return v2
}

/// @layout.struct name=test.main.Path size=32 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
/// @layout.variant name=type@17 size=40 align=8
/// @layout.discriminant owner=type@17 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@17 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@17 index=1 discriminant=1 payload_offset=8
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.Clone.clone<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }>",
        r#"
type test.main.Path {
    steps: Array<int32>;
}

@nocopy
@languageItem("memory.Clone")
type Clone;

function test.main.Clone.clone<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, 'a>(v0: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable>): variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; } {
    local l0: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable>
    local l1: variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }
    local l2: boolean, readonly

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable> = load l0
    v2: uint1 = variant.tag.load (*v1)
    switch v2, b1, 0 => b3

b1:
    v4: boolean = false
    store l2, v4
    jump b2

b2:
    v5: boolean = load l2
    branch v5 => b4 | b5

b3:
    v3: boolean = true
    store l2, v3
    jump b2

b4:
    v6: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable> = load l0
    v7: ref<test.main.Path, borrowed, 'a, immutable> = address ((*v6) as 0)
    v8: ref<test.main.Path, borrowed, 'a, immutable> = address (*v7)
    v9: test.main.Path = call.witness test.main.Path, Clone, Clone.clone(v8): (ref<test.main.Path, borrowed, 'a, immutable>) => test.main.Path
    v10: variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; } = variant.new 0, v9
    store l1, v10
    jump b6

b5:
    v11: ref<variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; }, borrowed, 'a, immutable> = load l0
    v12: ref<int32, borrowed, 'a, immutable> = address ((*v11) as 1)
    v13: ref<int32, borrowed, 'a, immutable> = address (*v12)
    v14: int32 = call Integer.Clone.clone<int32>(v13): (ref<int32, borrowed, 'a, immutable>) => int32
    v15: variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; } = variant.new 1, v14
    store l1, v15
    jump b6

b6:
    v16: variant<uint1> { 0uint1 = test.main.Path; 1uint1 = int32; } = load l1
    return v16
}

/// @layout.struct name=test.main.Path size=32 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
/// @layout.variant name=type@17 size=40 align=8
/// @layout.discriminant owner=type@17 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@17 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@17 index=1 discriminant=1 payload_offset=8
"#,
    );
}
