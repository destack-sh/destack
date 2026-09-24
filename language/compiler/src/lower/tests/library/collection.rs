use crate::tests::TestSession;

/// Collect a set of integers from an array through the default key equality.
#[test]
fn test_lower_a_set_collected_from_an_integer_array() {
    let session = TestSession::single(
        r#"
import { Set } from "destack:collections";

function collect(values: int32[]): Set<int32> {
    return Set.from(values);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.collect",
        r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

type Equality<T>;

@nocopy
@languageItem("collections.Set")
type Set<T, E>;

@nocopy
@languageItem("iter.Iterable")
type Iterable<T>;

function test.main.collect(v0: ref<Array<int32>, managed, mutable, local>): ref<Set<int32, Equality<int32>>, managed, mutable, local> {
    local l0: ref<Array<int32>, managed, mutable, local>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    store l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = load l0
    v2: dynamic<Iterable<int32>, managed, mutable, local> = dynamic.bind v1, ref<Array<int32>, managed, mutable, local>
    v3: Set<int32, Equality<int32>> = call Set.from<int32, Equality<int32>>(v2): (dynamic<Iterable<int32>, managed, mutable, local>) => Set<int32, Equality<int32>>
    v4: ref<Set<int32, Equality<int32>>, managed, mutable, local> = new.complete v3
    return v4
}
"#,
    );
}

/// Test set membership of an integer through the default key equality.
#[test]
fn test_lower_a_set_membership_test_through_default_equality() {
    let session = TestSession::single(
        r#"
import { Set } from "destack:collections";

function has(values: Set<int32>, value: int32): boolean {
    return values.has(value);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.has",
        r#"
type Equality<T>;

@nocopy
@languageItem("collections.Set")
type Set<T, E>;

function test.main.has(v0: ref<Set<int32, Equality<int32>>, managed, mutable, local>, v1: int32): boolean {
    local l0: ref<Set<int32, Equality<int32>>, managed, mutable, local>
    local l1: int32

entry(v0: ref<Set<int32, Equality<int32>>, managed, mutable, local>, v1: int32):
    store l0, v0
    store l1, v1
    v2: ref<Set<int32, Equality<int32>>, managed, mutable, local> = load l0
    v3: int32 = load l1
    v4: ref<Set<int32, Equality<int32>>, borrowed, 'managed, readonly> = cast.bit v2 -> ref<Set<int32, Equality<int32>>, borrowed, 'managed, readonly>
    v5: boolean = call Set.has<int32, Equality<int32>, int32, readonly>(v4, v3): (ref<Set<int32, Equality<int32>>, borrowed, 'managed, readonly>, int32) => boolean
    return v5
}
"#,
    );
}

/// Sort an array in place through a mutable receiver with a number comparator.
#[test]
fn test_lower_a_sort_with_a_number_comparator() {
    let session = TestSession::single(
        r#"
function order(values: float64[]): void {
    values.sort((left, right) => left - right);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.order",
        r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

function test.main.order(v0: ref<Array<float64>, managed, mutable, local>): void {
    local l0: ref<Array<float64>, managed, mutable, local>

entry(v0: ref<Array<float64>, managed, mutable, local>):
    store l0, v0
    v1: ref<Array<float64>, managed, mutable, local> = load l0
    v2: ptr<void, readonly> = null
    v3: function<(float64, float64) => float64, repeatable, managed, mutable, local> = function.bind test.main.order.closure#0, v2
    v4: ref<Array<float64>, managed, mutable, local> = call Array.sort<float64>(v1, v3): (ref<Array<float64>, managed, mutable, local>, function<(float64, float64) => float64, repeatable, managed, mutable, local>) => ref<Array<float64>, managed, mutable, local>
    return
}
"#,
    );
}

/// Iterate an immutable borrow of an array whose elements hold owned storage.
#[test]
fn test_lower_an_iteration_over_an_immutable_array_borrow() {
    let session = TestSession::single(
        r#"
struct Label {
    values: ^int32[];
}

function count(labels: &immutable Label[]): isize {
    let total: isize = 0;
    for (const label of labels) {
        total += label.values.length;
    }

    return total;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.count",
        r#"
type test.main.Label {
    values: Array<int32>;
}

@nocopy
@languageItem("collections.Array")
type Array<T>;

type ArrayIterator<T, 'a, A: Access>;

@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

function test.main.count<'a>(v0: ref<Array<test.main.Label>, borrowed, 'a, immutable>): isize {
    local l0: ref<Array<test.main.Label>, borrowed, 'a, immutable>
    local l1: isize
    local l2: ArrayIterator<test.main.Label, 'a, immutable>
    local l3: ref<test.main.Label, borrowed, 'frame, immutable>
    local l4: ref<test.main.Label, borrowed, 'a, immutable>

entry(v0: ref<Array<test.main.Label>, borrowed, 'a, immutable>):
    store l0, v0
    v1: isize = 0
    store l1, v1
    v2: ref<Array<test.main.Label>, borrowed, 'a, immutable> = load l0
    v3: ArrayIterator<test.main.Label, 'a, immutable> = call Array.Iterable.iterator<test.main.Label, 'a, immutable>(v2): (ref<Array<test.main.Label>, borrowed, 'a, immutable>) => ArrayIterator<test.main.Label, 'a, immutable>
    store l2, v3
    jump b1

b1:
    v4: ref<ArrayIterator<test.main.Label, 'frame, immutable>, borrowed, 'frame, mutable> = address l2
    v5: IteratorResult<ref<test.main.Label, borrowed, 'frame, immutable>, void> = call ArrayIterator.Iterator.next<test.main.Label, 'a, immutable>(v4): (ref<ArrayIterator<test.main.Label, 'frame, immutable>, borrowed, 'frame, mutable>) => IteratorResult<ref<test.main.Label, borrowed, 'frame, immutable>, void>
    v6: variant<uint1> { 0uint1 = IteratorYield<ref<test.main.Label, borrowed, 'frame, immutable>>; 1uint1 = IteratorReturn<void>; } = field.get v5, 0
    variant.switch v6, 0 => b2, 1 => b3

b2:
    v7: IteratorYield<ref<test.main.Label, borrowed, 'frame, immutable>> = variant.payload v6, 0
    v8: ref<test.main.Label, borrowed, 'frame, immutable> = field.get v7, 1
    store l3, v8
    v9: ref<test.main.Label, borrowed, 'frame, immutable> = load l3
    store l4, v9
    v10: isize = load l1
    v11: ref<test.main.Label, borrowed, 'a, immutable> = load l4
    v12: ref<Array<int32>, borrowed, 'a, readonly> = address (*v11).0
    v13: isize = call Array.length.get<int32>(v12): (ref<Array<int32>, borrowed, 'a, readonly>) => isize
    v14: isize = add v10, v13
    store l1, v14
    jump b1

b3:
    v15: isize = load l1
    return v15
}

/// @layout.struct name=test.main.Label size=32 align=8
/// @layout.field owner=test.main.Label index=0 name=values offset=0 size=32 align=8
/// @layout.variant name=type@68 size=16 align=8
/// @layout.discriminant owner=type@68 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@68 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@68 index=1 discriminant=1 payload_offset=8
"#,
    );
}
