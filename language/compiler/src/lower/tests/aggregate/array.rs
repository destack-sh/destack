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

    session.assert_mir_function("main.ds", "test.main.build", r#"
@languageItem("collections.Array")
type Array<T>;

@languageItem("iter.Iterator")
type Iterator<T>;

@copy
@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

@copy
@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@copy
@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

function test.main.build(v0: ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>): ref<Array<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, managed, mutable, local> {
    local l0: ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>
    local l1: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local>
    local l2: usize
    local l3: dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>
    local l4: usize
    local l5: usize

entry(v0: ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>):
    local.set l0, v0
    v1: int32 = 0
    v2: variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; } = variant.new 1, v1
    v3: usize = 1
    v4: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, v3
    local.set l1, v4
    local.set l2, v3
    v5: usize = 0
    v6: ref<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v4, v5
    store v6, v2
    v7: ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local> = local.get l0
    v8: dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local> = call Array.Iterable.iterator<ref<Array<int32>, managed, mutable, local>>(v7): (ref<Array<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>) => dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local>
    local.set l3, v8
    jump b1

b1:
    v9: dynamic<Iterator<ref<Array<int32>, managed, mutable, local>>, managed, mutable, local> = local.get l3
    v10: IteratorResult<ref<Array<int32>, managed, mutable, local>, void> = call.dynamic v9, Iterator<ref<Array<int32>, managed, mutable, local>>, 0(): () => IteratorResult<ref<Array<int32>, managed, mutable, local>, void>
    v11: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<ref<Array<int32>, managed, mutable, local>>; } = field.get v10, 0
    variant.switch v11, 1 => b2, 0 => b3

b2:
    v12: IteratorYield<ref<Array<int32>, managed, mutable, local>> = variant.payload v11, 1
    v13: ref<Array<int32>, managed, mutable, local> = field.get v12, 1
    v14: variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; } = variant.new 0, v13
    v15: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = local.get l1
    v16: usize = slice.length v15
    v17: usize = local.get l2
    v18: boolean = eq v17, v16
    branch v18 => b4 | b5

b3:
    v37: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = local.get l1
    v38: usize = slice.length v37
    v39: usize = local.get l2
    v40: boolean = eq v39, v38
    branch v40 => b10 | b9

b4:
    v19: usize = add v16, v16
    v20: usize = 1
    v21: usize = add v19, v20
    v22: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = local.get l1
    v23: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, v21
    v24: usize = local.get l2
    v25: usize = 0
    local.set l4, v25
    jump b6

b5:
    v33: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = local.get l1
    v34: ref<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v33, v17
    store v34, v14
    v35: usize = 1
    v36: usize = add v17, v35
    local.set l2, v36
    jump b1

b6:
    v26: usize = local.get l4
    v27: boolean = lt v26, v24
    branch v27 => b7 | b8

b7:
    v28: ref<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }, borrowed, 'frame, mutable, local> = element.address v22, v26
    v29: variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; } = load v28
    v30: ref<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v23, v26
    store v30, v29
    v31: usize = 1
    v32: usize = add v26, v31
    local.set l4, v32
    jump b6

b8:
    release v22
    local.set l1, v23
    jump b5

b9:
    v41: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = local.get l1
    v42: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, v39
    v43: usize = local.get l2
    v44: usize = 0
    local.set l5, v44
    jump b11

b10:
    v52: slice<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, unique, mutable, local> = local.get l1
    v53: slice<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }, unique, mutable, local> = new.complete v52
    v54: Array<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>(v53): (slice<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }, unique, mutable, local>) => Array<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>
    v55: ref<Array<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, managed, mutable, local> = new.complete v54
    return v55

b11:
    v45: usize = local.get l5
    v46: boolean = lt v45, v43
    branch v46 => b12 | b13

b12:
    v47: ref<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }, borrowed, 'frame, mutable, local> = element.address v41, v45
    v48: variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; } = load v47
    v49: ref<uninit<variant<uint1> { 0uint1 = ref<Array<int32>, managed, readonly, local>; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v42, v45
    store v49, v48
    v50: usize = 1
    v51: usize = add v45, v50
    local.set l5, v51
    jump b11

b13:
    release v41
    local.set l1, v42
    jump b10
}

/// @layout.variant name=type@26 size=16 align=8
/// @layout.discriminant owner=type@26 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@26 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@26 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@139 size=16 align=8
/// @layout.discriminant owner=type@139 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@139 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@139 index=1 discriminant=1 payload_offset=8
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

    session.assert_mir_function("main.ds", "test.main.build", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.build(): Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }> {
entry:
    v0: int32 = 1
    v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v0
    v2: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 0
    v3: int32 = 2
    v4: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v3
    v5: usize = 3
    v6: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, v5
    v7: usize = 0
    v8: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v6, v7
    store v8, v1
    v9: usize = 1
    v10: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v6, v9
    store v10, v2
    v11: usize = 2
    v12: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v6, v11
    store v12, v4
    v13: slice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, unique, mutable, local> = new.complete v6
    v14: Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>(v13): (slice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, unique, mutable, local>) => Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>
    return v14
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

    session.assert_mir_function("main.ds", "test.main.build", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.build(): Array<int32> {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: usize = 3
    v4: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v3
    v5: usize = 0
    v6: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v4, v5
    store v6, v0
    v7: usize = 1
    v8: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v4, v7
    store v8, v1
    v9: usize = 2
    v10: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v4, v9
    store v10, v2
    v11: slice<int32, unique, mutable, local> = new.complete v4
    v12: Array<int32> = call arrayFromOwnedSlice<int32>(v11): (slice<int32, unique, mutable, local>) => Array<int32>
    return v12
}
"#);
}

#[test]
fn test_lower_pushes_onto_an_owned_array() {
    let session = TestSession::single(
        r#"
import { Array } from "destack:collections";

function fill(): int32 {
    let values: ^Array<int32> = Array.new();

    values.push(1);

    return values.pop() ?? 0;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.fill", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.fill(): int32 {
    local l0: Array<int32>
    local l1: int32, readonly

entry:
    v0: Array<int32> = call Array.new<int32>(): () => Array<int32>
    local.set l0, v0
    v1: int32 = 1
    v2: usize = 1
    v3: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v2
    v4: usize = 0
    v5: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v3, v4
    store v5, v1
    v6: slice<int32, unique, mutable, local> = new.complete v3
    v7: Array<int32> = call arrayFromOwnedSlice<int32>(v6): (slice<int32, unique, mutable, local>) => Array<int32>
    v8: ref<Array<int32>, managed, mutable, local> = new.complete v7
    v9: ref<Array<int32>, borrowed, 'frame, mutable, local> = local.address l0
    v10: isize = call Array.push<int32>(v9, v8): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>, ref<Array<int32>, managed, mutable, local>) => isize
    v11: ref<Array<int32>, borrowed, 'frame, mutable, local> = local.address l0
    v12: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = call Array.pop<int32>(v11): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>) => variant<uint1> { 0uint1 = void; 1uint1 = int32; }
    variant.switch v12, 0 => b2, else b1

b1:
    v13: int32 = variant.payload v12, 1
    local.set l1, v13
    jump b3

b2:
    v14: int32 = 0
    local.set l1, v14
    jump b3

b3:
    v15: int32 = local.get l1
    return v15
}

/// @layout.variant name=type@77 size=8 align=4
/// @layout.discriminant owner=type@77 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@77 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@77 index=1 discriminant=1 payload_offset=4
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
        "main.ds",
        "test.main.zeros",
        r#"
function test.main.zeros(): [int32; 4] {
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
import { Range } from "destack:range";

function span(start: isize, end: isize): Range<isize> {
    return start..end;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.span",
        r#"
@copy
@languageItem("range.Range")
type Range<T>;

function test.main.span(v0: isize, v1: isize): Range<isize> {
    local l0: isize
    local l1: isize

entry(v0: isize, v1: isize):
    local.set l0, v0
    local.set l1, v1
    v2: isize = local.get l0
    v3: isize = local.get l1
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
        "main.ds",
        "test.main.main",
        r#"
@languageItem("collections.Array")
type Array<T>;

@copy
type test.main.Path {
    steps: Array<int32>;
}

function test.main.main(): test.main.Path {
    local l0: test.main.Path

entry:
    v0: usize = 0
    v1: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v0
    v2: slice<int32, unique, mutable, local> = new.complete v1
    v3: Array<int32> = call arrayFromOwnedSlice<int32>(v2): (slice<int32, unique, mutable, local>) => Array<int32>
    v4: test.main.Path = aggregate (v3)
    local.set l0, v4
    v5: test.main.Path = local.get l0
    return v5
}

/// @layout.struct name=test.main.Path size=32 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
"#,
    );
}
