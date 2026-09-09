use crate::tests::TestSession;

#[test]
fn test_lower_while_accumulation_through_locals() {
    let session = TestSession::single(
        r#"
function sum(n: int32): int32 {
    let total: int32 = 0;
    let i: int32 = 0;
    while (i < n) {
        total = total + i;
        i = i + 1;
    }
    return total;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.sum",
        r#"
function test.main.sum(v0: int32): int32 {
    local l0: int32
    local l1: int32
    local l2: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = 0
    local.set l1, v1
    v2: int32 = 0
    local.set l2, v2
    jump b1

b1:
    v3: int32 = local.get l2
    v4: int32 = local.get l0
    v5: boolean = lt v3, v4
    branch v5 => b2 | b3

b2:
    v6: int32 = local.get l1
    v7: int32 = local.get l2
    v8: int32 = add v6, v7
    local.set l1, v8
    v9: int32 = local.get l2
    v10: int32 = 1
    v11: int32 = add v9, v10
    local.set l2, v11
    jump b1

b3:
    v12: int32 = local.get l1
    return v12
}
"#,
    );
}

#[test]
fn test_lower_break_out_of_unbounded_while() {
    let session = TestSession::single(
        r#"
function firstOver(limit: int32): int32 {
    let i: int32 = 0;
    while (true) {
        if (i * i > limit) {
            break;
        }
        i = i + 1;
    }
    return i;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.firstOver",
        r#"
function test.main.firstOver(v0: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = 0
    local.set l1, v1
    jump b1

b1:
    v2: boolean = true
    branch v2 => b2 | b3

b2:
    v3: int32 = local.get l1
    v4: int32 = local.get l1
    v5: int32 = mul v3, v4
    v6: int32 = local.get l0
    v7: boolean = gt v5, v6
    branch v7 => b4 | b5

b3:
    v11: int32 = local.get l1
    return v11

b4:
    jump b3

b5:
    v8: int32 = local.get l1
    v9: int32 = 1
    v10: int32 = add v8, v9
    local.set l1, v10
    jump b1
}
"#,
    );
}

#[test]
fn test_lower_for_loop_with_update_increment() {
    let session = TestSession::single(
        r#"
function sum(n: int32): int32 {
    let total: int32 = 0;
    for (let i: int32 = 0; i < n; i++) {
        total += i;
    }
    return total;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.sum",
        r#"
function test.main.sum(v0: int32): int32 {
    local l0: int32
    local l1: int32
    local l2: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = 0
    local.set l1, v1
    v2: int32 = 0
    local.set l2, v2
    jump b1

b1:
    v3: int32 = local.get l2
    v4: int32 = local.get l0
    v5: boolean = lt v3, v4
    branch v5 => b2 | b4

b2:
    v6: int32 = local.get l1
    v7: int32 = local.get l2
    v8: int32 = add v6, v7
    local.set l1, v8
    jump b3

b3:
    v9: int32 = local.get l2
    v10: int32 = 1
    v11: int32 = add v9, v10
    local.set l2, v11
    jump b1

b4:
    v12: int32 = local.get l1
    return v12
}
"#,
    );
}

#[test]
fn test_run_a_do_while_body_before_its_condition() {
    let session = TestSession::single(
        r#"
function drain(n: int64): int64 {
    let left = n;
    do {
        left -= 1;
    } while (left > 0);
    return left;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.drain",
        r#"
function test.main.drain(v0: int64): int64 {
    local l0: int64
    local l1: int64

entry(v0: int64):
    local.set l0, v0
    v1: int64 = local.get l0
    local.set l1, v1
    jump b2

b1:
    v2: int64 = local.get l1
    v3: int64 = 0
    v4: boolean = gt v2, v3
    branch v4 => b2 | b3

b2:
    v5: int64 = local.get l1
    v6: int64 = 1
    v7: int64 = sub v5, v6
    local.set l1, v7
    jump b1

b3:
    v8: int64 = local.get l1
    return v8
}
"#,
    );
}

#[test]
fn test_lower_unconditional_loop_with_break() {
    let session = TestSession::single(
        r#"
function next(seed: int32): int32 {
    let value = seed;
    loop {
        value += 7;
        if (value > 100) {
            break;
        }
    }
    return value;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.next",
        r#"
function test.main.next(v0: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    local.set l1, v1
    jump b1

b1:
    v2: int32 = local.get l1
    v3: int32 = 7
    v4: int32 = add v2, v3
    local.set l1, v4
    v5: int32 = local.get l1
    v6: int32 = 100
    v7: boolean = gt v5, v6
    branch v7 => b3 | b4

b2:
    v8: int32 = local.get l1
    return v8

b3:
    jump b2

b4:
    jump b1
}
"#,
    );
}

#[test]
fn test_exit_an_outer_loop_through_a_labeled_break() {
    let session = TestSession::single(
        r#"
function find(limit: int32): int32 {
    let hits: int32 = 0;
    outer: for (let i: int32 = 0; i < limit; i++) {
        for (let j: int32 = 0; j < limit; j++) {
            if (i * j > limit) {
                break outer;
            }
            hits += 1;
        }
    }
    return hits;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.find",
        r#"
function test.main.find(v0: int32): int32 {
    local l0: int32
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = 0
    local.set l1, v1
    v2: int32 = 0
    local.set l2, v2
    jump b1

b1:
    v3: int32 = local.get l2
    v4: int32 = local.get l0
    v5: boolean = lt v3, v4
    branch v5 => b2 | b4

b2:
    v6: int32 = 0
    local.set l3, v6
    jump b5

b3:
    v21: int32 = local.get l2
    v22: int32 = 1
    v23: int32 = add v21, v22
    local.set l2, v23
    jump b1

b4:
    v24: int32 = local.get l1
    return v24

b5:
    v7: int32 = local.get l3
    v8: int32 = local.get l0
    v9: boolean = lt v7, v8
    branch v9 => b6 | b8

b6:
    v10: int32 = local.get l2
    v11: int32 = local.get l3
    v12: int32 = mul v10, v11
    v13: int32 = local.get l0
    v14: boolean = gt v12, v13
    branch v14 => b9 | b10

b7:
    v18: int32 = local.get l3
    v19: int32 = 1
    v20: int32 = add v18, v19
    local.set l3, v20
    jump b5

b8:
    jump b3

b9:
    jump b4

b10:
    v15: int32 = local.get l1
    v16: int32 = 1
    v17: int32 = add v15, v16
    local.set l1, v17
    jump b7
}
"#,
    );
}

#[test]
fn test_lower_a_for_of_loop_through_the_iterator_protocol() {
    let session = TestSession::single(
        r#"
function sum(values: int32[]): int32 {
    let total: int32 = 0;
    for (const value of values) {
        total += value;
    }

    return total;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.sum", r#"
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

function test.main.sum(v0: ref<Array<int32>, managed, mutable, local>): int32 {
    local l0: ref<Array<int32>, managed, mutable, local>
    local l1: int32
    local l2: dynamic<Iterator<int32>, managed, mutable, local>
    local l3: int32
    local l4: int32

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: int32 = 0
    local.set l1, v1
    v2: ref<Array<int32>, managed, mutable, local> = local.get l0
    v3: dynamic<Iterator<int32>, managed, mutable, local> = call Array.Iterable.iterator<int32>(v2): (ref<Array<int32>, managed, mutable, local>) => dynamic<Iterator<int32>, managed, mutable, local>
    local.set l2, v3
    jump b1

b1:
    v4: dynamic<Iterator<int32>, managed, mutable, local> = local.get l2
    v5: IteratorResult<int32, void> = call.dynamic v4, Iterator<int32>, 0(): () => IteratorResult<int32, void>
    v6: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<int32>; } = field.get v5, 0
    variant.switch v6, 1 => b2, 0 => b3

b2:
    v7: IteratorYield<int32> = variant.payload v6, 1
    v8: int32 = field.get v7, 1
    local.set l3, v8
    v9: int32 = local.get l3
    local.set l4, v9
    v10: int32 = local.get l1
    v11: int32 = local.get l4
    v12: int32 = add v10, v11
    local.set l1, v12
    jump b1

b3:
    v13: int32 = local.get l1
    return v13
}

/// @layout.variant name=type@102 size=8 align=4
/// @layout.discriminant owner=type@102 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@102 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@102 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_a_sequence_destructure_through_the_sequence_protocol() {
    let session = TestSession::single(
        r#"
function split(values: int32[]): int32 {
    const [first, second, ...rest] = values;

    return first + second;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.split", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.split(v0: ref<Array<int32>, managed, mutable, local>): int32 {
    local l0: ref<Array<int32>, managed, mutable, local>
    local l1: ref<Array<int32>, managed, mutable, local>
    local l2: ref<Array<int32>, managed, mutable, local>, readonly
    local l3: int32
    local l4: ref<Array<int32>, managed, mutable, local>, readonly
    local l5: int32
    local l6: Array<int32>
    local l7: Array<int32>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = local.get l0
    local.set l1, v1
    v2: ref<Array<int32>, managed, mutable, local> = local.get l1
    local.set l2, v2
    v3: ref<Array<int32>, borrowed, 'frame, mutable, local> = local.address l2
    v4: isize = 0
    v5: ref<int32, borrowed, 'frame, mutable, local> = call Array.Index.index<int32, mutable>(v3, v4): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>, isize) => ref<int32, borrowed, 'a, mutable, local>
    v6: int32 = load v5
    local.set l3, v6
    v7: ref<Array<int32>, managed, mutable, local> = local.get l1
    local.set l4, v7
    v8: ref<Array<int32>, borrowed, 'frame, mutable, local> = local.address l4
    v9: isize = 1
    v10: ref<int32, borrowed, 'frame, mutable, local> = call Array.Index.index<int32, mutable>(v8, v9): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>, isize) => ref<int32, borrowed, 'a, mutable, local>
    v11: int32 = load v10
    local.set l5, v11
    v12: ref<Array<int32>, managed, mutable, local> = local.get l1
    v13: isize = 2
    v14: variant<uint1> { 0uint1 = void; 1uint1 = isize; } = variant.new 0
    v15: Array<int32> = call Array.Sequence.rest<int32>(v12, v13, v14): (ref<Array<int32>, managed, mutable, local>, isize, variant<uint1> { 0uint1 = void; 1uint1 = isize; }) => Array<int32>
    local.set l6, v15
    v16: Array<int32> = local.get l6
    local.set l7, v16
    v17: int32 = local.get l3
    v18: int32 = local.get l5
    v19: int32 = add v17, v18
    return v19
}

/// @layout.variant name=type@82 size=16 align=8
/// @layout.discriminant owner=type@82 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@82 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@82 index=1 discriminant=1 payload_offset=8
"#);
}

#[test]
fn test_lower_a_loop_in_value_position_through_its_break_join() {
    let session = TestSession::single(
        r#"
function first(count: int32): int32 {
    let step = count;
    let found = loop {
        step -= 1;

        if (step > 0) {
            break (step);
        }
    };

    return found;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.first",
        r#"
function test.main.first(v0: int32): int32 {
    local l0: int32
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    local.set l1, v1
    jump b1

b1:
    v2: int32 = local.get l1
    v3: int32 = 1
    v4: int32 = sub v2, v3
    local.set l1, v4
    v5: int32 = local.get l1
    v6: int32 = 0
    v7: boolean = gt v5, v6
    branch v7 => b3 | b4

b2:
    v9: int32 = local.get l2
    local.set l3, v9
    v10: int32 = local.get l3
    return v10

b3:
    v8: int32 = local.get l1
    local.set l2, v8
    jump b2

b4:
    jump b1
}
"#,
    );
}

#[test]
fn test_lower_a_for_of_over_a_readonly_borrow_of_an_owned_array() {
    let session = TestSession::single(
        r#"
function total(items: ^int64[]): int64 {
    let total = 0;
    for (const item of &readonly items) {
        total += 1;
    }
    return total;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.total",
        r#"
@languageItem("collections.Array")
type Array<T>;

@copy
type ArrayIterator<T, 'a, access A>;

@copy
@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

@copy
@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@copy
@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

function test.main.total(v0: Array<int64>): int64 {
    local l0: Array<int64>
    local l1: int64
    local l2: ArrayIterator<int64, 'frame & local, readonly>
    local l3: ref<int64, borrowed, 'frame, readonly, local>
    local l4: ref<int64, borrowed, 'frame, readonly, local>

entry(v0: Array<int64>):
    local.set l0, v0
    v1: int64 = 0
    local.set l1, v1
    v2: ref<Array<int64>, borrowed, 'frame, readonly, local> = local.address l0
    v3: ArrayIterator<int64, 'frame & local, readonly> = call Array.Iterable.iterator<int64, readonly>(v2): <'a>(ref<Array<int64>, borrowed, 'a, readonly, local>) => ArrayIterator<int64, 'a & local, readonly>
    local.set l2, v3
    jump b1

b1:
    v4: ref<ArrayIterator<int64, 'frame & local, readonly>, borrowed, 'frame, mutable, local> = local.address l2
    v5: IteratorResult<ref<int64, borrowed, 'frame, readonly, local>, void> = call ArrayIterator.Iterator.next<int64, readonly>(v4): <'a, 'b>(ref<ArrayIterator<int64, 'a & local, readonly>, borrowed, 'b, mutable, local>) => IteratorResult<ref<int64, borrowed, 'a, readonly, local>, void>
    v6: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<ref<int64, borrowed, 'frame, readonly, local>>; } = field.get v5, 0
    variant.switch v6, 1 => b2, 0 => b3

b2:
    v7: IteratorYield<ref<int64, borrowed, 'frame, readonly, local>> = variant.payload v6, 1
    v8: ref<int64, borrowed, 'frame, readonly, local> = field.get v7, 1
    local.set l3, v8
    v9: ref<int64, borrowed, 'frame, readonly, local> = local.get l3
    local.set l4, v9
    v10: int64 = local.get l1
    v11: int64 = 1
    v12: int64 = add v10, v11
    local.set l1, v12
    jump b1

b3:
    v13: int64 = local.get l1
    return v13
}

/// @layout.variant name=type@110 size=16 align=8
/// @layout.discriminant owner=type@110 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@110 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@110 index=1 discriminant=1 payload_offset=8
"#,
    );
}
