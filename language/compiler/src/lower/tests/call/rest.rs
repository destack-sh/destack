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

    session.assert_mir_function("main.ds", "test.main.forward", r#"
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

function test.main.forward(v0: ref<Array<int32>, managed, mutable, local>): void {
    local l0: ref<Array<int32>, managed, mutable, local>
    local l1: slice<uninit<int32>, unique, mutable, local>
    local l2: usize
    local l3: dynamic<Iterator<int32>, managed, mutable, local>
    local l4: usize
    local l5: usize
    local l6: dynamic<Iterator<int32>, managed, mutable, local>
    local l7: usize
    local l8: usize

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: int32 = 0
    v2: usize = 1
    v3: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v2
    local.set l1, v3
    local.set l2, v2
    v4: usize = 0
    v5: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v3, v4
    store v5, v1
    v6: ref<Array<int32>, managed, mutable, local> = local.get l0
    v7: dynamic<Iterator<int32>, managed, mutable, local> = call Array.Iterable.iterator<int32>(v6): (ref<Array<int32>, managed, mutable, local>) => dynamic<Iterator<int32>, managed, mutable, local>
    local.set l3, v7
    jump b1

b1:
    v8: dynamic<Iterator<int32>, managed, mutable, local> = local.get l3
    v9: IteratorResult<int32, void> = call.dynamic v8, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v10: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<int32>; } = field.get v9, 0
    variant.switch v10, 1 => b2, 0 => b3

b2:
    v11: IteratorYield<int32> = variant.payload v10, 1
    v12: int32 = field.get v11, 1
    v13: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v14: usize = slice.length v13
    v15: usize = local.get l2
    v16: boolean = eq v15, v14
    branch v16 => b4 | b5

b3:
    v35: ref<Array<int32>, managed, mutable, local> = local.get l0
    v36: int32 = call test.main.change(v35): (ref<Array<int32>, managed, mutable, local>) => int32
    v37: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v38: usize = slice.length v37
    v39: usize = local.get l2
    v40: boolean = eq v39, v38
    branch v40 => b9 | b10

b4:
    v17: usize = add v14, v14
    v18: usize = 1
    v19: usize = add v17, v18
    v20: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v21: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v19
    v22: usize = local.get l2
    v23: usize = 0
    local.set l4, v23
    jump b6

b5:
    v31: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v32: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v31, v15
    store v32, v12
    v33: usize = 1
    v34: usize = add v15, v33
    local.set l2, v34
    jump b1

b6:
    v24: usize = local.get l4
    v25: boolean = lt v24, v22
    branch v25 => b7 | b8

b7:
    v26: ref<int32, borrowed, 'frame, mutable, local> = element.address v20, v24
    v27: int32 = load v26
    v28: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v21, v24
    store v28, v27
    v29: usize = 1
    v30: usize = add v24, v29
    local.set l4, v30
    jump b6

b8:
    release v20
    local.set l1, v21
    jump b5

b9:
    v41: usize = add v38, v38
    v42: usize = 1
    v43: usize = add v41, v42
    v44: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v45: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v43
    v46: usize = local.get l2
    v47: usize = 0
    local.set l5, v47
    jump b11

b10:
    v55: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v56: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v55, v39
    store v56, v36
    v57: usize = 1
    v58: usize = add v39, v57
    local.set l2, v58
    v59: int32 = 4
    v60: int32 = 5
    v61: usize = 2
    v62: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v61
    v63: usize = 0
    v64: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v62, v63
    store v64, v59
    v65: usize = 1
    v66: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v62, v65
    store v66, v60
    v67: slice<int32, unique, mutable, local> = new.complete v62
    v68: Array<int32> = call arrayFromOwnedSlice<int32>(v67): (slice<int32, unique, mutable, local>) => Array<int32>
    v69: ref<Array<int32>, managed, mutable, local> = new.complete v68
    v70: dynamic<Iterator<int32>, managed, mutable, local> = call Array.Iterable.iterator<int32>(v69): (ref<Array<int32>, managed, mutable, local>) => dynamic<Iterator<int32>, managed, mutable, local>
    local.set l6, v70
    jump b14

b11:
    v48: usize = local.get l5
    v49: boolean = lt v48, v46
    branch v49 => b12 | b13

b12:
    v50: ref<int32, borrowed, 'frame, mutable, local> = element.address v44, v48
    v51: int32 = load v50
    v52: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v45, v48
    store v52, v51
    v53: usize = 1
    v54: usize = add v48, v53
    local.set l5, v54
    jump b11

b13:
    release v44
    local.set l1, v45
    jump b10

b14:
    v71: dynamic<Iterator<int32>, managed, mutable, local> = local.get l6
    v72: IteratorResult<int32, void> = call.dynamic v71, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v73: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<int32>; } = field.get v72, 0
    variant.switch v73, 1 => b15, 0 => b16

b15:
    v74: IteratorYield<int32> = variant.payload v73, 1
    v75: int32 = field.get v74, 1
    v76: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v77: usize = slice.length v76
    v78: usize = local.get l2
    v79: boolean = eq v78, v77
    branch v79 => b17 | b18

b16:
    v98: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v99: usize = slice.length v98
    v100: usize = local.get l2
    v101: boolean = eq v100, v99
    branch v101 => b23 | b22

b17:
    v80: usize = add v77, v77
    v81: usize = 1
    v82: usize = add v80, v81
    v83: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v84: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v82
    v85: usize = local.get l2
    v86: usize = 0
    local.set l7, v86
    jump b19

b18:
    v94: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v95: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v94, v78
    store v95, v75
    v96: usize = 1
    v97: usize = add v78, v96
    local.set l2, v97
    jump b14

b19:
    v87: usize = local.get l7
    v88: boolean = lt v87, v85
    branch v88 => b20 | b21

b20:
    v89: ref<int32, borrowed, 'frame, mutable, local> = element.address v83, v87
    v90: int32 = load v89
    v91: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v84, v87
    store v91, v90
    v92: usize = 1
    v93: usize = add v87, v92
    local.set l7, v93
    jump b19

b21:
    release v83
    local.set l1, v84
    jump b18

b22:
    v102: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v103: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v100
    v104: usize = local.get l2
    v105: usize = 0
    local.set l8, v105
    jump b24

b23:
    v113: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v114: slice<int32, unique, mutable, local> = new.complete v113
    call test.main.take(v114): (slice<int32, unique, mutable, local>) => void
    return

b24:
    v106: usize = local.get l8
    v107: boolean = lt v106, v104
    branch v107 => b25 | b26

b25:
    v108: ref<int32, borrowed, 'frame, mutable, local> = element.address v102, v106
    v109: int32 = load v108
    v110: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v103, v106
    store v110, v109
    v111: usize = 1
    v112: usize = add v106, v111
    local.set l8, v112
    jump b24

b26:
    release v102
    local.set l1, v103
    jump b23
}

/// @layout.variant name=type@113 size=8 align=4
/// @layout.discriminant owner=type@113 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@113 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@113 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_function("main.ds", "test.main.forward", r#"
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

function test.main.forward(v0: slice<int32, unique, mutable, local>): isize {
    local l0: slice<int32, unique, mutable, local>
    local l1: slice<uninit<int32>, unique, mutable, local>
    local l2: usize
    local l3: slice<int32, unique, mutable, local>, readonly
    local l4: dynamic<Iterator<int32>, managed, mutable, local>
    local l5: usize
    local l6: usize
    local l7: slice<int32, unique, mutable, local>, readonly

entry(v0: slice<int32, unique, mutable, local>):
    local.set l0, v0
    v1: usize = 0
    v2: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v1
    local.set l1, v2
    local.set l2, v1
    v3: slice<int32, unique, mutable, local> = local.get l0
    local.set l3, v3
    v4: slice<int32, borrowed, 'frame, readonly, local> = local.address l3
    v5: dynamic<Iterator<int32>, managed, mutable, local> = call Slice.Iterable.iterator<int32>(v4): <'a>(slice<int32, borrowed, 'a, readonly, local>) => dynamic<Iterator<int32>, managed, mutable, local>
    local.set l4, v5
    jump b1

b1:
    v6: dynamic<Iterator<int32>, managed, mutable, local> = local.get l4
    v7: IteratorResult<int32, void> = call.dynamic v6, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v8: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<int32>; } = field.get v7, 0
    variant.switch v8, 1 => b2, 0 => b3

b2:
    v9: IteratorYield<int32> = variant.payload v8, 1
    v10: int32 = field.get v9, 1
    v11: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v12: usize = slice.length v11
    v13: usize = local.get l2
    v14: boolean = eq v13, v12
    branch v14 => b4 | b5

b3:
    v33: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v34: usize = slice.length v33
    v35: usize = local.get l2
    v36: boolean = eq v35, v34
    branch v36 => b10 | b9

b4:
    v15: usize = add v12, v12
    v16: usize = 1
    v17: usize = add v15, v16
    v18: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v19: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v17
    v20: usize = local.get l2
    v21: usize = 0
    local.set l5, v21
    jump b6

b5:
    v29: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v30: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v29, v13
    store v30, v10
    v31: usize = 1
    v32: usize = add v13, v31
    local.set l2, v32
    jump b1

b6:
    v22: usize = local.get l5
    v23: boolean = lt v22, v20
    branch v23 => b7 | b8

b7:
    v24: ref<int32, borrowed, 'frame, mutable, local> = element.address v18, v22
    v25: int32 = load v24
    v26: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v19, v22
    store v26, v25
    v27: usize = 1
    v28: usize = add v22, v27
    local.set l5, v28
    jump b6

b8:
    release v18
    local.set l1, v19
    jump b5

b9:
    v37: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v38: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v35
    v39: usize = local.get l2
    v40: usize = 0
    local.set l6, v40
    jump b11

b10:
    v48: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v49: slice<int32, unique, mutable, local> = new.complete v48
    local.set l7, v49
    v50: slice<int32, unique, mutable, local> = local.get l7
    v51: usize = 0
    v52: usize = slice.length v50
    v53: slice<int32, borrowed, 'frame, readonly, local> = slice.view v50, v51, v52
    v54: isize = call test.main.total(v53): <'a>(slice<int32, borrowed, 'a, readonly, local>) => isize
    return v54

b11:
    v41: usize = local.get l6
    v42: boolean = lt v41, v39
    branch v42 => b12 | b13

b12:
    v43: ref<int32, borrowed, 'frame, mutable, local> = element.address v37, v41
    v44: int32 = load v43
    v45: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v38, v41
    store v45, v44
    v46: usize = 1
    v47: usize = add v41, v46
    local.set l6, v47
    jump b11

b13:
    release v37
    local.set l1, v38
    jump b10
}

/// @layout.variant name=type@97 size=8 align=4
/// @layout.discriminant owner=type@97 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@97 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@97 index=1 discriminant=1 payload_offset=4
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
        "main.ds",
        "test.main.main",
        r#"
function test.main.main(): isize {
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
    v12: isize = call test.main.total(v11): (slice<int32, unique, mutable, local>) => isize
    return v12
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
        "main.ds",
        "test.main.main",
        r#"
function test.main.main(): isize {
    local l0: [int32; 3], readonly

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    local.set l0, v3
    v4: ref<[int32; 3], borrowed, 'frame, readonly, frame> = local.address l0
    v5: uint64 = 0
    v6: usize = 3
    v7: slice<int32, borrowed, 'frame, readonly, frame> = slice.view v4, v5, v6
    v8: isize = call test.main.total(v7): <'a>(slice<int32, borrowed, 'a, readonly, local>) => isize
    return v8
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

    session.assert_mir_function("main.ds", "test.main.total", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.total(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = local.get l0
    v2: ref<Array<int32>, borrowed, 'managed, readonly, local> = cast.bit v1 -> ref<Array<int32>, borrowed, 'managed, readonly, local>
    v3: isize = call Array.length.get<int32>(v2): <'a>(ref<Array<int32>, borrowed, 'a, readonly, local>) => isize
    return v3
}
"#);

    session.assert_mir_function("main.ds", "test.main.main", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.main(): isize {
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
    v13: ref<Array<int32>, managed, mutable, local> = new.complete v12
    v14: isize = call test.main.total(v13): (ref<Array<int32>, managed, mutable, local>) => isize
    return v14
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

    session.assert_mir_function("main.ds", "test.main.sum", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.sum(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = local.get l0
    v2: ref<Array<int32>, borrowed, 'managed, readonly, local> = cast.bit v1 -> ref<Array<int32>, borrowed, 'managed, readonly, local>
    v3: isize = call Array.length.get<int32>(v2): <'a>(ref<Array<int32>, borrowed, 'a, readonly, local>) => isize
    return v3
}
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.forward",
        r#"
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

function test.main.forward(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>
    local l1: slice<uninit<int32>, unique, mutable, local>
    local l2: usize
    local l3: dynamic<Iterator<int32>, managed, mutable, local>
    local l4: usize
    local l5: usize

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: usize = 0
    v2: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v1
    local.set l1, v2
    local.set l2, v1
    v3: ref<Array<int32>, managed, mutable, local> = local.get l0
    v4: dynamic<Iterator<int32>, managed, mutable, local> = call Array.Iterable.iterator<int32>(v3): (ref<Array<int32>, managed, mutable, local>) => dynamic<Iterator<int32>, managed, mutable, local>
    local.set l3, v4
    jump b1

b1:
    v5: dynamic<Iterator<int32>, managed, mutable, local> = local.get l3
    v6: IteratorResult<int32, void> = call.dynamic v5, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v7: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<int32>; } = field.get v6, 0
    variant.switch v7, 1 => b2, 0 => b3

b2:
    v8: IteratorYield<int32> = variant.payload v7, 1
    v9: int32 = field.get v8, 1
    v10: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v11: usize = slice.length v10
    v12: usize = local.get l2
    v13: boolean = eq v12, v11
    branch v13 => b4 | b5

b3:
    v32: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v33: usize = slice.length v32
    v34: usize = local.get l2
    v35: boolean = eq v34, v33
    branch v35 => b10 | b9

b4:
    v14: usize = add v11, v11
    v15: usize = 1
    v16: usize = add v14, v15
    v17: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v18: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v16
    v19: usize = local.get l2
    v20: usize = 0
    local.set l4, v20
    jump b6

b5:
    v28: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v29: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v28, v12
    store v29, v9
    v30: usize = 1
    v31: usize = add v12, v30
    local.set l2, v31
    jump b1

b6:
    v21: usize = local.get l4
    v22: boolean = lt v21, v19
    branch v22 => b7 | b8

b7:
    v23: ref<int32, borrowed, 'frame, mutable, local> = element.address v17, v21
    v24: int32 = load v23
    v25: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v18, v21
    store v25, v24
    v26: usize = 1
    v27: usize = add v21, v26
    local.set l4, v27
    jump b6

b8:
    release v17
    local.set l1, v18
    jump b5

b9:
    v36: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v37: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v34
    v38: usize = local.get l2
    v39: usize = 0
    local.set l5, v39
    jump b11

b10:
    v47: slice<uninit<int32>, unique, mutable, local> = local.get l1
    v48: slice<int32, unique, mutable, local> = new.complete v47
    v49: Array<int32> = call arrayFromOwnedSlice<int32>(v48): (slice<int32, unique, mutable, local>) => Array<int32>
    v50: ref<Array<int32>, managed, mutable, local> = new.complete v49
    v51: isize = call test.main.sum(v50): (ref<Array<int32>, managed, mutable, local>) => isize
    return v51

b11:
    v40: usize = local.get l5
    v41: boolean = lt v40, v38
    branch v41 => b12 | b13

b12:
    v42: ref<int32, borrowed, 'frame, mutable, local> = element.address v36, v40
    v43: int32 = load v42
    v44: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v37, v40
    store v44, v43
    v45: usize = 1
    v46: usize = add v40, v45
    local.set l5, v46
    jump b11

b13:
    release v36
    local.set l1, v37
    jump b10
}

/// @layout.variant name=type@120 size=8 align=4
/// @layout.discriminant owner=type@120 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@120 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@120 index=1 discriminant=1 payload_offset=4
"#,
    );
}
