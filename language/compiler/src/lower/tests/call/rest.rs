use crate::tests::TestSession;

/// Rest arguments consume each spread before evaluating the following argument.
#[test]
fn test_pack_spread_arguments_in_order() {
    let session = TestSession::single(
        r#"
declare function take(...values: ^[int32]): void;

declare function change(values: int32[]): int32;

function forward(values: int32[]): void {
    take(0, ...values, change(values), ...[4, 5]);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.forward", r#"
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

function test.main.forward(v0: ref<Array<int32>, managed, mutable, local>): void {
    local l0: ref<Array<int32>, managed, mutable, local>
    local l1: slice<uninit<int32>, unique, mutable>
    local l2: usize
    local l3: dynamic<Iterator<int32>, managed, mutable, local>
    local l4: usize
    local l5: usize
    local l6: dynamic<Iterator<int32>, managed, mutable, local>
    local l7: usize
    local l8: usize

entry(v0: ref<Array<int32>, managed, mutable, local>):
    store l0, v0
    v1: int32 = 0
    v2: usize = 1
    v3: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v2, local
    store l1, v3
    store l2, v2
    v4: usize = 0
    store (*l1)[v4], v1
    v5: ref<Array<int32>, managed, mutable, local> = load l0
    v6: dynamic<Iterator<int32>, managed, mutable, local> = call Array.Iterable.iterator<int32>(v5): (ref<Array<int32>, managed, mutable, local>) => dynamic<Iterator<int32>, managed, mutable, local>
    store l3, v6
    jump b1

b1:
    v7: dynamic<Iterator<int32>, managed, mutable, local> = load l3
    v8: dynamic<Iterator<int32>, borrowed, 'managed, mutable> = cast.bit v7 -> dynamic<Iterator<int32>, borrowed, 'managed, mutable>
    v9: IteratorResult<int32, void> = call.dynamic v8, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v10: variant<uint1> { 0uint1 = IteratorYield<int32>; 1uint1 = IteratorReturn<void>; } = field.get v9, 0
    variant.switch v10, 0 => b2, 1 => b3

b2:
    v11: IteratorYield<int32> = variant.payload v10, 0
    v12: int32 = field.get v11, 1
    v13: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v14: usize = slice.length v13
    v15: usize = load l2
    v16: boolean = eq v15, v14
    branch v16 => b4 | b5

b3:
    v31: ref<Array<int32>, managed, mutable, local> = load l0
    v32: int32 = call test.main.change(v31): (ref<Array<int32>, managed, mutable, local>) => int32
    v33: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v34: usize = slice.length v33
    v35: usize = load l2
    v36: boolean = eq v35, v34
    branch v36 => b9 | b10

b4:
    v17: usize = add v14, v14
    v18: usize = 1
    v19: usize = add v17, v18
    v20: slice<uninit<int32>, unique, mutable> = load l1
    v21: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v19, local
    v22: usize = load l2
    v23: usize = 0
    store l4, v23
    jump b6

b5:
    store (*l1)[v15], v12
    v29: usize = 1
    v30: usize = add v15, v29
    store l2, v30
    jump b1

b6:
    v24: usize = load l4
    v25: boolean = lt v24, v22
    branch v25 => b7 | b8

b7:
    v26: int32 = load (*v20)[v24]
    store (*v21)[v24], v26
    v27: usize = 1
    v28: usize = add v24, v27
    store l4, v28
    jump b6

b8:
    release v20
    store l1, v21
    jump b5

b9:
    v37: usize = add v34, v34
    v38: usize = 1
    v39: usize = add v37, v38
    v40: slice<uninit<int32>, unique, mutable> = load l1
    v41: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v39, local
    v42: usize = load l2
    v43: usize = 0
    store l5, v43
    jump b11

b10:
    store (*l1)[v35], v32
    v49: usize = 1
    v50: usize = add v35, v49
    store l2, v50
    v51: int32 = 4
    v52: int32 = 5
    v53: usize = 2
    v54: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v53, local
    v55: usize = 0
    store (*v54)[v55], v51
    v56: usize = 1
    store (*v54)[v56], v52
    v57: slice<int32, unique, mutable> = new.complete v54
    v58: Array<int32> = call arrayFromOwnedSlice<int32>(v57): (slice<int32, unique, mutable>) => Array<int32>
    v59: dynamic<Iterator<int32>, managed, mutable, local> = call Array.Iterable.iterator<int32>(v58): (Array<int32>) => dynamic<Iterator<int32>, managed, mutable, local>
    store l6, v59
    jump b14

b11:
    v44: usize = load l5
    v45: boolean = lt v44, v42
    branch v45 => b12 | b13

b12:
    v46: int32 = load (*v40)[v44]
    store (*v41)[v44], v46
    v47: usize = 1
    v48: usize = add v44, v47
    store l5, v48
    jump b11

b13:
    release v40
    store l1, v41
    jump b10

b14:
    v60: dynamic<Iterator<int32>, managed, mutable, local> = load l6
    v61: dynamic<Iterator<int32>, borrowed, 'managed, mutable> = cast.bit v60 -> dynamic<Iterator<int32>, borrowed, 'managed, mutable>
    v62: IteratorResult<int32, void> = call.dynamic v61, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v63: variant<uint1> { 0uint1 = IteratorYield<int32>; 1uint1 = IteratorReturn<void>; } = field.get v62, 0
    variant.switch v63, 0 => b15, 1 => b16

b15:
    v64: IteratorYield<int32> = variant.payload v63, 0
    v65: int32 = field.get v64, 1
    v66: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v67: usize = slice.length v66
    v68: usize = load l2
    v69: boolean = eq v68, v67
    branch v69 => b17 | b18

b16:
    v84: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v85: usize = slice.length v84
    v86: usize = load l2
    v87: boolean = eq v86, v85
    branch v87 => b23 | b22

b17:
    v70: usize = add v67, v67
    v71: usize = 1
    v72: usize = add v70, v71
    v73: slice<uninit<int32>, unique, mutable> = load l1
    v74: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v72, local
    v75: usize = load l2
    v76: usize = 0
    store l7, v76
    jump b19

b18:
    store (*l1)[v68], v65
    v82: usize = 1
    v83: usize = add v68, v82
    store l2, v83
    jump b14

b19:
    v77: usize = load l7
    v78: boolean = lt v77, v75
    branch v78 => b20 | b21

b20:
    v79: int32 = load (*v73)[v77]
    store (*v74)[v77], v79
    v80: usize = 1
    v81: usize = add v77, v80
    store l7, v81
    jump b19

b21:
    release v73
    store l1, v74
    jump b18

b22:
    v88: slice<uninit<int32>, unique, mutable> = load l1
    v89: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v86, local
    v90: usize = load l2
    v91: usize = 0
    store l8, v91
    jump b24

b23:
    v97: slice<uninit<int32>, unique, mutable> = load l1
    v98: slice<int32, unique, mutable> = new.complete v97
    call test.main.take(v98): (slice<int32, unique, mutable>) => void
    return

b24:
    v92: usize = load l8
    v93: boolean = lt v92, v90
    branch v93 => b25 | b26

b25:
    v94: int32 = load (*v88)[v92]
    store (*v89)[v92], v94
    v95: usize = 1
    v96: usize = add v92, v95
    store l8, v96
    jump b24

b26:
    release v88
    store l1, v89
    jump b23
}

/// @layout.variant name=type@123 size=8 align=4
/// @layout.discriminant owner=type@123 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@123 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@123 index=1 discriminant=1 payload_offset=4
"#);
}

/// A spread fills fresh storage for a borrowed rest parameter.
#[test]
fn test_spread_into_borrowed_rest_parameter() {
    let session = TestSession::single(
        r#"
function total(...values: &readonly [int32]): isize {
    return values.length;
}

function forward(values: ^[int32]): isize {
    return total(...values);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.forward", r#"
@nocopy
@languageItem("iter.Iterator")
type Iterator<T>;

@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

function test.main.forward(v0: slice<int32, unique, mutable>): isize {
    local l0: slice<int32, unique, mutable>
    local l1: slice<uninit<int32>, unique, mutable>
    local l2: usize
    local l3: dynamic<Iterator<int32>, managed, mutable, local>
    local l4: usize
    local l5: usize
    local l6: slice<int32, unique, mutable>, readonly

entry(v0: slice<int32, unique, mutable>):
    store l0, v0
    v1: usize = 0
    v2: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v1, local
    store l1, v2
    store l2, v1
    v3: slice<int32, unique, mutable> = load l0
    v4: slice<int32, borrowed, 'frame, readonly> = address (*v3)
    v5: dynamic<Iterator<int32>, managed, mutable, local> = call Slice.Iterable.iterator<int32>(v4): (slice<int32, borrowed, 'frame, readonly>) => dynamic<Iterator<int32>, managed, mutable, local>
    store l3, v5
    jump b1

b1:
    v6: dynamic<Iterator<int32>, managed, mutable, local> = load l3
    v7: dynamic<Iterator<int32>, borrowed, 'managed, mutable> = cast.bit v6 -> dynamic<Iterator<int32>, borrowed, 'managed, mutable>
    v8: IteratorResult<int32, void> = call.dynamic v7, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v9: variant<uint1> { 0uint1 = IteratorYield<int32>; 1uint1 = IteratorReturn<void>; } = field.get v8, 0
    variant.switch v9, 0 => b2, 1 => b3

b2:
    v10: IteratorYield<int32> = variant.payload v9, 0
    v11: int32 = field.get v10, 1
    v12: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v13: usize = slice.length v12
    v14: usize = load l2
    v15: boolean = eq v14, v13
    branch v15 => b4 | b5

b3:
    v30: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v31: usize = slice.length v30
    v32: usize = load l2
    v33: boolean = eq v32, v31
    branch v33 => b10 | b9

b4:
    v16: usize = add v13, v13
    v17: usize = 1
    v18: usize = add v16, v17
    v19: slice<uninit<int32>, unique, mutable> = load l1
    v20: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v18, local
    v21: usize = load l2
    v22: usize = 0
    store l4, v22
    jump b6

b5:
    store (*l1)[v14], v11
    v28: usize = 1
    v29: usize = add v14, v28
    store l2, v29
    jump b1

b6:
    v23: usize = load l4
    v24: boolean = lt v23, v21
    branch v24 => b7 | b8

b7:
    v25: int32 = load (*v19)[v23]
    store (*v20)[v23], v25
    v26: usize = 1
    v27: usize = add v23, v26
    store l4, v27
    jump b6

b8:
    release v19
    store l1, v20
    jump b5

b9:
    v34: slice<uninit<int32>, unique, mutable> = load l1
    v35: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v32, local
    v36: usize = load l2
    v37: usize = 0
    store l5, v37
    jump b11

b10:
    v43: slice<uninit<int32>, unique, mutable> = load l1
    v44: slice<int32, unique, mutable> = new.complete v43
    v45: usize = slice.length v44
    store l6, v44
    v46: usize = 0
    v47: slice<int32, borrowed, 'frame, readonly> = address (*l6)[v46; v45]
    v48: isize = call test.main.total(v47): (slice<int32, borrowed, 'frame, readonly>) => isize
    return v48

b11:
    v38: usize = load l5
    v39: boolean = lt v38, v36
    branch v39 => b12 | b13

b12:
    v40: int32 = load (*v34)[v38]
    store (*v35)[v38], v40
    v41: usize = 1
    v42: usize = add v38, v41
    store l5, v42
    jump b11

b13:
    release v34
    store l1, v35
    jump b10
}

/// @layout.variant name=type@127 size=8 align=4
/// @layout.discriminant owner=type@127 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@127 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@127 index=1 discriminant=1 payload_offset=4
"#);
}

/// Pack positional arguments into an owned rest slice.
#[test]
fn test_pack_owned_rest_slice() {
    let session = TestSession::single(
        r#"
function total(...values: ^[int32]): isize {
    return values.length;
}

function main(): isize {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.main",
        r#"
function test.main.main(): isize {
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
    v9: isize = call test.main.total(v8): (slice<int32, unique, mutable>) => isize
    return v9
}
"#,
    );
}

/// Pack positional arguments into a borrowed rest slice.
#[test]
fn test_pack_borrowed_rest_slice() {
    let session = TestSession::single(
        r#"
function total(...values: &readonly [int32]): isize {
    return values.length;
}

function main(): isize {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.main",
        r#"
function test.main.main(): isize {
    local l0: [int32; 3], readonly

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    store l0, v3
    v4: usize = 0
    v5: usize = 3
    v6: slice<int32, borrowed, 'frame, readonly> = address l0[v4; v5]
    v7: isize = call test.main.total(v6): (slice<int32, borrowed, 'frame, readonly>) => isize
    return v7
}
"#,
    );
}

/// Pack positional arguments into a rest array.
#[test]
fn test_pack_rest_array() {
    let session = TestSession::single(
        r#"
function total(...values: int32[]): isize {
    return values.length;
}

function main(): isize {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.total", r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

function test.main.total(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    store l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = load l0
    v2: ref<Array<int32>, borrowed, 'managed, readonly> = cast.bit v1 -> ref<Array<int32>, borrowed, 'managed, readonly>
    v3: isize = call Array.length.get<int32>(v2): (ref<Array<int32>, borrowed, 'managed, readonly>) => isize
    return v3
}
"#);

    session.assert_mir_function("main.tspp", "test.main.main", r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

function test.main.main(): isize {
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
    v10: ref<Array<int32>, managed, mutable, local> = new.complete v9
    v11: isize = call test.main.total(v10): (ref<Array<int32>, managed, mutable, local>) => isize
    return v11
}
"#);
}

/// Copy spread elements into a fresh rest array.
#[test]
fn test_spread_into_rest_array() {
    let session = TestSession::single(
        r#"
function sum(...values: int32[]): isize {
    return values.length;
}

function forward(values: int32[]): isize {
    return sum(...values);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.sum", r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

function test.main.sum(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    store l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = load l0
    v2: ref<Array<int32>, borrowed, 'managed, readonly> = cast.bit v1 -> ref<Array<int32>, borrowed, 'managed, readonly>
    v3: isize = call Array.length.get<int32>(v2): (ref<Array<int32>, borrowed, 'managed, readonly>) => isize
    return v3
}
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.forward",
        r#"
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

function test.main.forward(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>
    local l1: slice<uninit<int32>, unique, mutable>
    local l2: usize
    local l3: dynamic<Iterator<int32>, managed, mutable, local>
    local l4: usize
    local l5: usize

entry(v0: ref<Array<int32>, managed, mutable, local>):
    store l0, v0
    v1: usize = 0
    v2: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v1, local
    store l1, v2
    store l2, v1
    v3: ref<Array<int32>, managed, mutable, local> = load l0
    v4: dynamic<Iterator<int32>, managed, mutable, local> = call Array.Iterable.iterator<int32>(v3): (ref<Array<int32>, managed, mutable, local>) => dynamic<Iterator<int32>, managed, mutable, local>
    store l3, v4
    jump b1

b1:
    v5: dynamic<Iterator<int32>, managed, mutable, local> = load l3
    v6: dynamic<Iterator<int32>, borrowed, 'managed, mutable> = cast.bit v5 -> dynamic<Iterator<int32>, borrowed, 'managed, mutable>
    v7: IteratorResult<int32, void> = call.dynamic v6, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v8: variant<uint1> { 0uint1 = IteratorYield<int32>; 1uint1 = IteratorReturn<void>; } = field.get v7, 0
    variant.switch v8, 0 => b2, 1 => b3

b2:
    v9: IteratorYield<int32> = variant.payload v8, 0
    v10: int32 = field.get v9, 1
    v11: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v12: usize = slice.length v11
    v13: usize = load l2
    v14: boolean = eq v13, v12
    branch v14 => b4 | b5

b3:
    v29: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v30: usize = slice.length v29
    v31: usize = load l2
    v32: boolean = eq v31, v30
    branch v32 => b10 | b9

b4:
    v15: usize = add v12, v12
    v16: usize = 1
    v17: usize = add v15, v16
    v18: slice<uninit<int32>, unique, mutable> = load l1
    v19: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v17, local
    v20: usize = load l2
    v21: usize = 0
    store l4, v21
    jump b6

b5:
    store (*l1)[v13], v10
    v27: usize = 1
    v28: usize = add v13, v27
    store l2, v28
    jump b1

b6:
    v22: usize = load l4
    v23: boolean = lt v22, v20
    branch v23 => b7 | b8

b7:
    v24: int32 = load (*v18)[v22]
    store (*v19)[v22], v24
    v25: usize = 1
    v26: usize = add v22, v25
    store l4, v26
    jump b6

b8:
    release v18
    store l1, v19
    jump b5

b9:
    v33: slice<uninit<int32>, unique, mutable> = load l1
    v34: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v31, local
    v35: usize = load l2
    v36: usize = 0
    store l5, v36
    jump b11

b10:
    v42: slice<uninit<int32>, unique, mutable> = load l1
    v43: slice<int32, unique, mutable> = new.complete v42
    v44: Array<int32> = call arrayFromOwnedSlice<int32>(v43): (slice<int32, unique, mutable>) => Array<int32>
    v45: ref<Array<int32>, managed, mutable, local> = new.complete v44
    v46: isize = call test.main.sum(v45): (ref<Array<int32>, managed, mutable, local>) => isize
    return v46

b11:
    v37: usize = load l5
    v38: boolean = lt v37, v35
    branch v38 => b12 | b13

b12:
    v39: int32 = load (*v33)[v37]
    store (*v34)[v37], v39
    v40: usize = 1
    v41: usize = add v37, v40
    store l5, v41
    jump b11

b13:
    release v33
    store l1, v34
    jump b10
}

/// @layout.variant name=type@131 size=8 align=4
/// @layout.discriminant owner=type@131 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@131 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@131 index=1 discriminant=1 payload_offset=4
"#,
    );
}
