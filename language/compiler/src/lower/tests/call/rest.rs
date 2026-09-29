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

export function test.main.forward(v0: ref<Array<int32>, managed, mutable, local>): void {
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
    v9: ref<Iterator<int32>, borrowed, 'managed, mutable> = dynamic.payload v8
    v10: IteratorResult<int32, void> = call.dynamic v8, Iterator<int32>, 0(v9): (ref<Iterator<int32>, borrowed, 'managed, mutable>) => IteratorResult<int32, void>
    v11: variant<uint1> { 0uint1 = IteratorYield<int32>; 1uint1 = IteratorReturn<void>; } = field.get v10, 0
    variant.switch v11, 0 => b2, 1 => b3

b2:
    v12: IteratorYield<int32> = variant.payload v11, 0
    v13: int32 = field.get v12, 1
    v14: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v15: usize = slice.length v14
    v16: usize = load l2
    v17: boolean = eq v16, v15
    branch v17 => b4 | b5

b3:
    v32: ref<Array<int32>, managed, mutable, local> = load l0
    v33: int32 = call test.main.change(v32): (ref<Array<int32>, managed, mutable, local>) => int32
    v34: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v35: usize = slice.length v34
    v36: usize = load l2
    v37: boolean = eq v36, v35
    branch v37 => b9 | b10

b4:
    v18: usize = add v15, v15
    v19: usize = 1
    v20: usize = add v18, v19
    v21: slice<uninit<int32>, unique, mutable> = load l1
    v22: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v20, local
    v23: usize = load l2
    v24: usize = 0
    store l4, v24
    jump b6

b5:
    store (*l1)[v16], v13
    v30: usize = 1
    v31: usize = add v16, v30
    store l2, v31
    jump b1

b6:
    v25: usize = load l4
    v26: boolean = lt v25, v23
    branch v26 => b7 | b8

b7:
    v27: int32 = load (*v21)[v25]
    store (*v22)[v25], v27
    v28: usize = 1
    v29: usize = add v25, v28
    store l4, v29
    jump b6

b8:
    release v21
    store l1, v22
    jump b5

b9:
    v38: usize = add v35, v35
    v39: usize = 1
    v40: usize = add v38, v39
    v41: slice<uninit<int32>, unique, mutable> = load l1
    v42: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v40, local
    v43: usize = load l2
    v44: usize = 0
    store l5, v44
    jump b11

b10:
    store (*l1)[v36], v33
    v50: usize = 1
    v51: usize = add v36, v50
    store l2, v51
    v52: int32 = 4
    v53: int32 = 5
    v54: usize = 2
    v55: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v54, local
    v56: usize = 0
    store (*v55)[v56], v52
    v57: usize = 1
    store (*v55)[v57], v53
    v58: slice<int32, unique, mutable> = new.complete v55
    v59: Array<int32> = call arrayFromOwnedSlice<int32>(v58): (slice<int32, unique, mutable>) => Array<int32>
    v60: dynamic<Iterator<int32>, managed, mutable, local> = call Array.Iterable.iterator<int32>(v59): (Array<int32>) => dynamic<Iterator<int32>, managed, mutable, local>
    store l6, v60
    jump b14

b11:
    v45: usize = load l5
    v46: boolean = lt v45, v43
    branch v46 => b12 | b13

b12:
    v47: int32 = load (*v41)[v45]
    store (*v42)[v45], v47
    v48: usize = 1
    v49: usize = add v45, v48
    store l5, v49
    jump b11

b13:
    release v41
    store l1, v42
    jump b10

b14:
    v61: dynamic<Iterator<int32>, managed, mutable, local> = load l6
    v62: dynamic<Iterator<int32>, borrowed, 'managed, mutable> = cast.bit v61 -> dynamic<Iterator<int32>, borrowed, 'managed, mutable>
    v63: ref<Iterator<int32>, borrowed, 'managed, mutable> = dynamic.payload v62
    v64: IteratorResult<int32, void> = call.dynamic v62, Iterator<int32>, 0(v63): (ref<Iterator<int32>, borrowed, 'managed, mutable>) => IteratorResult<int32, void>
    v65: variant<uint1> { 0uint1 = IteratorYield<int32>; 1uint1 = IteratorReturn<void>; } = field.get v64, 0
    variant.switch v65, 0 => b15, 1 => b16

b15:
    v66: IteratorYield<int32> = variant.payload v65, 0
    v67: int32 = field.get v66, 1
    v68: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v69: usize = slice.length v68
    v70: usize = load l2
    v71: boolean = eq v70, v69
    branch v71 => b17 | b18

b16:
    v86: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v87: usize = slice.length v86
    v88: usize = load l2
    v89: boolean = eq v88, v87
    branch v89 => b23 | b22

b17:
    v72: usize = add v69, v69
    v73: usize = 1
    v74: usize = add v72, v73
    v75: slice<uninit<int32>, unique, mutable> = load l1
    v76: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v74, local
    v77: usize = load l2
    v78: usize = 0
    store l7, v78
    jump b19

b18:
    store (*l1)[v70], v67
    v84: usize = 1
    v85: usize = add v70, v84
    store l2, v85
    jump b14

b19:
    v79: usize = load l7
    v80: boolean = lt v79, v77
    branch v80 => b20 | b21

b20:
    v81: int32 = load (*v75)[v79]
    store (*v76)[v79], v81
    v82: usize = 1
    v83: usize = add v79, v82
    store l7, v83
    jump b19

b21:
    release v75
    store l1, v76
    jump b18

b22:
    v90: slice<uninit<int32>, unique, mutable> = load l1
    v91: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v88, local
    v92: usize = load l2
    v93: usize = 0
    store l8, v93
    jump b24

b23:
    v99: slice<uninit<int32>, unique, mutable> = load l1
    v100: slice<int32, unique, mutable> = new.complete v99
    call test.main.take(v100): (slice<int32, unique, mutable>) => void
    return

b24:
    v94: usize = load l8
    v95: boolean = lt v94, v92
    branch v95 => b25 | b26

b25:
    v96: int32 = load (*v90)[v94]
    store (*v91)[v94], v96
    v97: usize = 1
    v98: usize = add v94, v97
    store l8, v98
    jump b24

b26:
    release v90
    store l1, v91
    jump b23
}

/// @layout.variant name=type@125 size=8 align=4
/// @layout.discriminant owner=type@125 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@125 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@125 index=1 discriminant=1 payload_offset=4
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

export function test.main.forward(v0: slice<int32, unique, mutable>): isize {
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
    v8: ref<Iterator<int32>, borrowed, 'managed, mutable> = dynamic.payload v7
    v9: IteratorResult<int32, void> = call.dynamic v7, Iterator<int32>, 0(v8): (ref<Iterator<int32>, borrowed, 'managed, mutable>) => IteratorResult<int32, void>
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
    v31: slice<uninit<int32>, borrowed, 'frame, readonly> = address (*l1)
    v32: usize = slice.length v31
    v33: usize = load l2
    v34: boolean = eq v33, v32
    branch v34 => b10 | b9

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
    v35: slice<uninit<int32>, unique, mutable> = load l1
    v36: slice<uninit<int32>, unique, mutable> = new.slice.uninit uninit<int32>, v33, local
    v37: usize = load l2
    v38: usize = 0
    store l5, v38
    jump b11

b10:
    v44: slice<uninit<int32>, unique, mutable> = load l1
    v45: slice<int32, unique, mutable> = new.complete v44
    v46: usize = slice.length v45
    store l6, v45
    v47: usize = 0
    v48: slice<int32, borrowed, 'frame, readonly> = address (*l6)[v47; v46]
    v49: isize = call test.main.total(v48): (slice<int32, borrowed, 'frame, readonly>) => isize
    return v49

b11:
    v39: usize = load l5
    v40: boolean = lt v39, v37
    branch v40 => b12 | b13

b12:
    v41: int32 = load (*v35)[v39]
    store (*v36)[v39], v41
    v42: usize = 1
    v43: usize = add v39, v42
    store l5, v43
    jump b11

b13:
    release v35
    store l1, v36
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
export function test.main.main(): isize {
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
export function test.main.main(): isize {
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

export function test.main.total(v0: ref<Array<int32>, managed, mutable, local>): isize {
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

export function test.main.main(): isize {
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
    v10: uninit<ref<Array<int32>, managed, mutable, local>> = new.uninit Array<int32>, local
    store (*v10), v9
    v11: ref<Array<int32>, managed, mutable, local> = new.complete v10
    v12: isize = call test.main.total(v11): (ref<Array<int32>, managed, mutable, local>) => isize
    return v12
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

export function test.main.sum(v0: ref<Array<int32>, managed, mutable, local>): isize {
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

export function test.main.forward(v0: ref<Array<int32>, managed, mutable, local>): isize {
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
    v7: ref<Iterator<int32>, borrowed, 'managed, mutable> = dynamic.payload v6
    v8: IteratorResult<int32, void> = call.dynamic v6, Iterator<int32>, 0(v7): (ref<Iterator<int32>, borrowed, 'managed, mutable>) => IteratorResult<int32, void>
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
    v45: Array<int32> = call arrayFromOwnedSlice<int32>(v44): (slice<int32, unique, mutable>) => Array<int32>
    v46: uninit<ref<Array<int32>, managed, mutable, local>> = new.uninit Array<int32>, local
    store (*v46), v45
    v47: ref<Array<int32>, managed, mutable, local> = new.complete v46
    v48: isize = call test.main.sum(v47): (ref<Array<int32>, managed, mutable, local>) => isize
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

/// @layout.variant name=type@133 size=8 align=4
/// @layout.discriminant owner=type@133 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@133 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@133 index=1 discriminant=1 payload_offset=4
"#,
    );
}
