use crate::tests::TestSession;

/// Lower a derived union equality dispatching on both members.
#[test]
fn test_lower_a_derived_union_equality() {
    let session = TestSession::single(
        r#"
import { PartialEqual } from "destack:ops";

struct Point {
    x: int32;
    y: int32;
}

function same<T: PartialEqual<T>>(left: &readonly T, right: &readonly T): boolean {
    return left.equal(right);
}

function compare(left: &readonly (Point | int32), right: &readonly (Point | int32)): boolean {
    return same(left, right);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.compare", r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.compare<'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local>
    local l1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local>

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local> = local.get l1
    v4: boolean = call test.main.same<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }>(v2, v3): <'a, 'b>(ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local>, ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local>) => boolean
    return v4
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@6 size=12 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.same<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }>",
        r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

shared function test.main.same<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, 'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local>): boolean;

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@6 size=12 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.PartialEqual.equal<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }>",
        r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.PartialEqual.equal<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, 'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local>
    local l1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local>
    local l2: boolean
    local l3: boolean, readonly
    local l4: boolean, readonly
    local l5: boolean, readonly

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local>, v1: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local>):
    local.set l0, v1
    local.set l1, v0
    v2: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local> = local.get l1
    v3: uint1 = variant.tag.load v2
    switch v3, b1, 0 => b3

b1:
    v5: boolean = false
    local.set l3, v5
    jump b2

b2:
    v6: boolean = local.get l3
    branch v6 => b4 | b5

b3:
    v4: boolean = true
    local.set l3, v4
    jump b2

b4:
    v7: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local> = local.get l0
    v8: uint1 = variant.tag.load v7
    switch v8, b7, 0 => b9

b5:
    v20: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local> = local.get l0
    v21: uint1 = variant.tag.load v20
    switch v21, b13, 1 => b15

b6:
    v33: boolean = local.get l2
    return v33

b7:
    v10: boolean = false
    local.set l4, v10
    jump b8

b8:
    v11: boolean = local.get l4
    branch v11 => b10 | b11

b9:
    v9: boolean = true
    local.set l4, v9
    jump b8

b10:
    v12: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local> = local.get l1
    v13: ref<test.main.Point, borrowed, 'a, readonly, local> = variant.payload.address v12, 0
    v14: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local> = local.get l0
    v15: ref<test.main.Point, borrowed, 'b, readonly, local> = variant.payload.address v14, 0
    v16: ref<test.main.Point, borrowed, 'b, readonly, local> = cast.bit v15 -> ref<test.main.Point, borrowed, 'b, readonly, local>
    v17: ref<test.main.Point, borrowed, 'a, readonly, local> = cast.bit v13 -> ref<test.main.Point, borrowed, 'a, readonly, local>
    v18: boolean = call test.main.PartialEqual.equal<test.main.Point>(v17, v16): <'a, 'b>(ref<test.main.Point, borrowed, 'a, readonly, local>, ref<test.main.Point, borrowed, 'b, readonly, local>) => boolean
    local.set l2, v18
    jump b12

b11:
    v19: boolean = false
    local.set l2, v19
    jump b12

b12:
    jump b6

b13:
    v23: boolean = false
    local.set l5, v23
    jump b14

b14:
    v24: boolean = local.get l5
    branch v24 => b16 | b17

b15:
    v22: boolean = true
    local.set l5, v22
    jump b14

b16:
    v25: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'a, readonly, local> = local.get l1
    v26: ref<int32, borrowed, 'a, readonly, local> = variant.payload.address v25, 1
    v27: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = int32; }, borrowed, 'b, readonly, local> = local.get l0
    v28: ref<int32, borrowed, 'b, readonly, local> = variant.payload.address v27, 1
    v29: ref<int32, borrowed, 'b, readonly, local> = cast.bit v28 -> ref<int32, borrowed, 'b, readonly, local>
    v30: ref<int32, borrowed, 'a, readonly, local> = cast.bit v26 -> ref<int32, borrowed, 'a, readonly, local>
    v31: boolean = call Integer.PartialEqual.equal<int32>(v30, v29): <'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, ref<int32, borrowed, 'b, readonly, local>) => boolean
    local.set l2, v31
    jump b18

b17:
    v32: boolean = false
    local.set l2, v32
    jump b18

b18:
    jump b6
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@6 size=12 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
"#,
    );

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

/// Lower a derived union hash writing the member position before its value.
#[test]
fn test_lower_a_derived_union_hash() {
    let session = TestSession::single(
        r#"
import { Hash, Hasher } from "destack:ops";

struct Point {
    x: int32;
    y: int32;
}

function digest<T: Hash>(value: &readonly T, state: &Hasher): void {
    value.hash(state);
}

function feed(value: &readonly (Point | null), state: &Hasher): void {
    digest(value, state);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.feed", r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

@languageItem("ops.Hasher")
type Hasher;

function test.main.feed<'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, readonly, local>, v1: ref<Hasher, borrowed, 'b, mutable, local>): void {
    local l0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, readonly, local>
    local l1: ref<Hasher, borrowed, 'b, mutable, local>

entry(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, readonly, local>, v1: ref<Hasher, borrowed, 'b, mutable, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<Hasher, borrowed, 'b, mutable, local> = local.get l1
    call test.main.digest<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }>(v2, v3): <'a, 'b>(ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, readonly, local>, ref<Hasher, borrowed, 'b, mutable, local>) => void
    return
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@7 size=12 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.digest<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }>",
        r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

@languageItem("ops.Hasher")
type Hasher;

shared function test.main.digest<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, 'a, 'b>(v0: ref<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, borrowed, 'a, readonly, local>, v1: ref<Hasher, borrowed, 'b, mutable, local>): void;

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@7 size=12 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Hash.hash<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }>",
        r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

@languageItem("ops.Hasher")
type Hasher;

function test.main.Hash.hash<variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, 'a>(v0: variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, v1: ref<Hasher, borrowed, 'a, mutable, local>): void {
    local l0: ref<Hasher, borrowed, 'a, mutable, local>
    local l1: variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }
    local l2: boolean, readonly

entry(v0: variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; }, v1: ref<Hasher, borrowed, 'a, mutable, local>):
    local.set l0, v1
    local.set l1, v0
    v2: variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; } = local.get l1
    variant.switch v2, 0 => b3, else b1

b1:
    v4: boolean = false
    local.set l2, v4
    jump b2

b2:
    v5: boolean = local.get l2
    branch v5 => b4 | b6

b3:
    v3: boolean = true
    local.set l2, v3
    jump b2

b4:
    v6: usize = 0
    v7: ref<Hasher, borrowed, 'a, mutable, local> = local.get l0
    call Integer.Hash.hash<usize>(v6, v7): <'a>(usize, ref<Hasher, borrowed, 'a, mutable, local>) => void
    v8: variant<uint1> { 0uint1 = test.main.Point; 1uint1 = null; } = local.get l1
    v9: test.main.Point = variant.payload v8, 0
    v10: ref<Hasher, borrowed, 'a, mutable, local> = local.get l0
    call test.main.Hash.hash<test.main.Point>(v9, v10): <'a>(test.main.Point, ref<Hasher, borrowed, 'a, mutable, local>) => void
    jump b5

b5:
    return

b6:
    v11: usize = 1
    v12: ref<Hasher, borrowed, 'a, mutable, local> = local.get l0
    call Integer.Hash.hash<usize>(v11, v12): <'a>(usize, ref<Hasher, borrowed, 'a, mutable, local>) => void
    jump b5
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.variant name=type@7 size=12 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Hash.hash<test.main.Point>", r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
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
    v5: test.main.Point = local.get l1
    v6: int32 = field.get v5, 1
    v7: ref<Hasher, borrowed, 'a, mutable, local> = local.get l0
    call Integer.Hash.hash<int32>(v6, v7): <'a>(int32, ref<Hasher, borrowed, 'a, mutable, local>) => void
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

function duplicate<T: Clone>(value: &readonly T): T {
    return value.clone();
}

function copy(value: &readonly (Path | int32)): Path | int32 {
    return duplicate(value);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.copy", r#"
@copy
type test.main.Path {
    steps: Array<int32>;
}

function test.main.copy<'a>(v0: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local>): variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; } {
    local l0: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local>

entry(v0: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local> = local.get l0
    v2: variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; } = call test.main.duplicate<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }>(v1): <'a>(ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local>) => variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }
    return v2
}

/// @layout.struct name=test.main.Path size=32 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
/// @layout.variant name=type@23 size=16 align=8
/// @layout.discriminant owner=type@23 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@23 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@23 index=1 discriminant=1 payload_offset=8
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.Clone.clone<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }>",
        r#"
@copy
type test.main.Path {
    steps: Array<int32>;
}

function test.main.Clone.clone<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, 'a>(v0: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local>): variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; } {
    local l0: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local>
    local l1: variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }
    local l2: boolean, readonly

entry(v0: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local> = local.get l0
    v2: uint1 = variant.tag.load v1
    switch v2, b1, 0 => b3

b1:
    v4: boolean = false
    local.set l2, v4
    jump b2

b2:
    v5: boolean = local.get l2
    branch v5 => b4 | b5

b3:
    v3: boolean = true
    local.set l2, v3
    jump b2

b4:
    v6: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local> = local.get l0
    v7: ref<test.main.Path, borrowed, 'a, readonly, local> = variant.payload.address v6, 0
    v8: ref<test.main.Path, borrowed, 'a, readonly, local> = cast.bit v7 -> ref<test.main.Path, borrowed, 'a, readonly, local>
    v9: test.main.Path = call test.main.Clone.clone<test.main.Path>(v8): <'a>(ref<test.main.Path, borrowed, 'a, readonly, local>) => test.main.Path
    v10: variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; } = variant.new 0, v9
    local.set l1, v10
    jump b6

b5:
    v11: ref<variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; }, borrowed, 'a, readonly, local> = local.get l0
    v12: ref<int32, borrowed, 'a, readonly, local> = variant.payload.address v11, 1
    v13: ref<int32, borrowed, 'a, readonly, local> = cast.bit v12 -> ref<int32, borrowed, 'a, readonly, local>
    v14: int32 = call Integer.Clone.clone<int32>(v13): <'a>(ref<int32, borrowed, 'a, readonly, local>) => int32
    v15: variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; } = variant.new 1, v14
    local.set l1, v15
    jump b6

b6:
    v16: variant<uint1> { 0uint1 = boxed ref<test.main.Path, unique, mutable, local>; 1uint1 = int32; } = local.get l1
    return v16
}

/// @layout.struct name=test.main.Path size=32 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
/// @layout.variant name=type@23 size=16 align=8
/// @layout.discriminant owner=type@23 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@23 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@23 index=1 discriminant=1 payload_offset=8
"#,
    );
}
