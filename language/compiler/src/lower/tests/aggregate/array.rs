use crate::tests::TestSession;

/// Array spreads convert each copied element to the destination element type.
#[test]
fn test_construct_array_with_spread_conversions() {
    let session = TestSession::single(
        r#"
function build(values: int32[][]): (int32 | readonly int32[])[] {
    return [0, ...values];
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.build", r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

@nocopy
@languageItem("iter.Iterator")
type Iterator<T>;

@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

export function test.main.build(v0: ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>): ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, managed, mutable, local> {
    local l0: ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>
    local l1: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, unique, mutable>
    local l2: usize
    local l3: dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>
    local l4: usize
    local l5: usize

entry(v0: ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>):
    store l0, v0
    v1: int32 = 0
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; } = variant.new 0, v1
    v3: usize = 1
    v4: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, v3, local
    store l1, v4
    store l2, v3
    v5: usize = 0
    store (*l1)[v5], v2
    v6: ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local> = load l0
    v7: dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local> = call Array.Iterable.iterator<ref<Array<int32>, managed, mutable, local>>(v6): (ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>) => dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>
    store l3, v7
    jump b1

b1:
    v8: dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local> = load l3
    v9: dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, borrowed, 'managed, mutable> = cast.bit v8 -> dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, borrowed, 'managed, mutable>
    v10: ref<Iterator<ref<Array<int32>, managed, mutable, local>>, borrowed, 'managed, mutable> = dynamic.payload v9
    v11: IteratorResult<ref<Array<int32>, managed, mutable, local>, void> = call.dynamic v9, Iterator<ref<Array<int32>, managed, mutable, local>>, 0(v10): (ref<Iterator<ref<Array<int32>, managed, mutable, local>>, borrowed, 'managed, mutable>) => IteratorResult<ref<Array<int32>, managed, mutable, local>, void>
    v12: variant<uint1> { 0uint1 = IteratorYield<ref<Array<int32>, managed, mutable, local>>; 1uint1 = IteratorReturn<void>; } = field.get v11, 0
    variant.switch v12, 0 => b2, 1 => b3

b2:
    v13: IteratorYield<ref<Array<int32>, managed, mutable, local>> = variant.payload v12, 0
    v14: ref<Array<int32>, managed, mutable, local> = field.get v13, 1
    v15: variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; } = variant.new 1, v14
    v16: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, borrowed, 'frame, readonly> = address (*l1)
    v17: usize = slice.length v16
    v18: usize = load l2
    v19: boolean = eq v18, v17
    branch v19 => b4 | b5

b3:
    v34: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, borrowed, 'frame, readonly> = address (*l1)
    v35: usize = slice.length v34
    v36: usize = load l2
    v37: boolean = eq v36, v35
    branch v37 => b10 | b9

b4:
    v20: usize = add v17, v17
    v21: usize = 1
    v22: usize = add v20, v21
    v23: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, unique, mutable> = load l1
    v24: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, v22, local
    v25: usize = load l2
    v26: usize = 0
    store l4, v26
    jump b6

b5:
    store (*l1)[v18], v15
    v32: usize = 1
    v33: usize = add v18, v32
    store l2, v33
    jump b1

b6:
    v27: usize = load l4
    v28: boolean = lt v27, v25
    branch v28 => b7 | b8

b7:
    v29: variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; } = load (*v23)[v27]
    store (*v24)[v27], v29
    v30: usize = 1
    v31: usize = add v27, v30
    store l4, v31
    jump b6

b8:
    release v23
    store l1, v24
    jump b5

b9:
    v38: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, unique, mutable> = load l1
    v39: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, v36, local
    v40: usize = load l2
    v41: usize = 0
    store l5, v41
    jump b11

b10:
    v47: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, unique, mutable> = load l1
    v48: slice<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }, unique, mutable> = new.complete v47
    v49: Array<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>(v48): (slice<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }, unique, mutable>) => Array<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>
    v50: uninit<ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, managed, mutable, local>> = new.uninit Array<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, local
    store (*v50), v49
    v51: ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; }>, managed, mutable, local> = new.complete v50
    return v51

b11:
    v42: usize = load l5
    v43: boolean = lt v42, v40
    branch v43 => b12 | b13

b12:
    v44: variant<uint1> { 0uint1 = int32; 1uint1 = ref<Array<int32>, managed, readonly, local>; } = load (*v38)[v42]
    store (*v39)[v42], v44
    v45: usize = 1
    v46: usize = add v42, v45
    store l5, v46
    jump b11

b13:
    release v38
    store l1, v39
    jump b10
}

/// @layout.variant name=type@15 size=16 align=8
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@143 size=8 align=8
/// @layout.discriminant owner=type@143 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@143 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@143 index=1 discriminant=1 payload_offset=0
"#);
}

/// Array holes occupy elements containing undefined.
#[test]
fn test_construct_array_with_holes() {
    let session = TestSession::single(
        r#"
function build(): ^(int32 | undefined)[] {
    return [1, , 2];
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.build", r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

export function test.main.build(): Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }> {
entry:
    v0: int32 = 1
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    v3: int32 = 2
    v4: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v3
    v5: usize = 3
    v6: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, v5, local
    v7: usize = 0
    store (*v6)[v7], v1
    v8: usize = 1
    store (*v6)[v8], v2
    v9: usize = 2
    store (*v6)[v9], v4
    v10: slice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, unique, mutable> = new.complete v6
    v11: Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>(v10): (slice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, unique, mutable>) => Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>
    return v11
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_array_literals_construct_arrays() {
    let session = TestSession::single(
        r#"
function build(): ^int32[] {
    return [1, 2, 3];
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.build", r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

export function test.main.build(): Array<int32> {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: usize = 3
    v4: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v3, local
    v5: usize = 0
    store (*v4)[v5], v0
    v6: usize = 1
    store (*v4)[v6], v1
    v7: usize = 2
    store (*v4)[v7], v2
    v8: slice<int32, unique, mutable> = new.complete v4
    v9: Array<int32> = call arrayFromOwnedSlice<int32>(v8): (slice<int32, unique, mutable>) => Array<int32>
    return v9
}
"#);
}

#[test]
fn test_lower_pushes_onto_an_owned_array() {
    let session = TestSession::single(
        r#"
import { Array } from "tspp:collections";

function fill(): int32 {
    let values: ^Array<int32> = Array.new();

    values.push(1);

    return values.pop() ?? 0;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.fill", r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

export function test.main.fill(): int32 {
    local l0: Array<int32>
    local l1: int32, readonly

entry:
    v0: Array<int32> = call Array.new<int32>(): () => Array<int32>
    store l0, v0
    v1: int32 = 1
    v2: usize = 1
    v3: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v2, local
    v4: usize = 0
    store (*v3)[v4], v1
    v5: slice<int32, unique, mutable> = new.complete v3
    v6: Array<int32> = call arrayFromOwnedSlice<int32>(v5): (slice<int32, unique, mutable>) => Array<int32>
    v7: uninit<ref<Array<int32>, managed, mutable, local>> = new.uninit Array<int32>, local
    store (*v7), v6
    v8: ref<Array<int32>, managed, mutable, local> = new.complete v7
    v9: ref<Array<int32>, borrowed, 'frame, mutable> = address l0
    v10: isize = call Array.push<int32>(v9, v8): (ref<Array<int32>, borrowed, 'frame, mutable>, ref<Array<int32>, managed, mutable, local>) => isize
    v11: ref<Array<int32>, borrowed, 'frame, mutable> = address l0
    v12: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = call Array.pop<int32>(v11): (ref<Array<int32>, borrowed, 'frame, mutable>) => variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    variant.switch v12, 1 => b2, else b1

b1:
    v13: int32 = variant.payload v12, 0
    store l1, v13
    jump b3

b2:
    v14: int32 = 0
    store l1, v14
    jump b3

b3:
    v15: int32 = load l1
    return v15
}

/// @layout.variant name=type@36 size=8 align=4
/// @layout.discriminant owner=type@36 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@36 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@36 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_a_repeated_fixed_array() {
    let session = TestSession::single(
        r#"
function zeros(): [int32; 4] {
    return [0; 4];
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.zeros",
        r#"
export function test.main.zeros(): [int32; 4] {
entry:
    v0: int32 = 0
    v1: [int32; 4] = aggregate (v0, v0, v0, v0)
    return v1
}
"#,
    );
}

#[test]
fn test_lower_a_range_to_its_language_family() {
    let session = TestSession::single(
        r#"
import { Range } from "tspp:range";

function span(start: isize, end: isize): Range<isize> {
    return start..end;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.span",
        r#"
@languageItem("range.Range")
type Range<T>;

export function test.main.span(v0: isize, v1: isize): Range<isize> {
    local l0: isize
    local l1: isize

entry(v0: isize, v1: isize):
    store l0, v0
    store l1, v1
    v2: isize = load l0
    v3: isize = load l1
    v4: Range<isize> = aggregate (v2, v3)
    return v4
}
"#,
    );
}

#[test]
fn test_lower_an_empty_array_literal_into_fresh_owned_storage() {
    let session = TestSession::single(
        r#"
struct Path {
    steps: ^Array<int32>;
}

function main(): Path {
    const path = Path { steps: [] };
    return path;
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.main",
        r#"
type test.main.Path {
    steps: Array<int32>;
}

@nocopy
@languageItem("collections.Array")
type Array<T>;

export function test.main.main(): test.main.Path {
    local l0: test.main.Path

entry:
    v0: usize = 0
    v1: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v0, local
    v2: slice<int32, unique, mutable> = new.complete v1
    v3: Array<int32> = call arrayFromOwnedSlice<int32>(v2): (slice<int32, unique, mutable>) => Array<int32>
    v4: test.main.Path = aggregate (v3)
    store l0, v4
    v5: test.main.Path = load l0
    return v5
}

/// @layout.struct name=test.main.Path size=40 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=40 align=8
"#,
    );
}
