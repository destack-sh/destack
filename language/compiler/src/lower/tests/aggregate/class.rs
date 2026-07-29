use crate::tests::TestSession;

#[test]
fn test_lower_struct_class_reference_cycle() {
    let session = TestSession::single(
        r#"
struct Wrapper {
    node: Node;
}

class Node {
    wrapper: Wrapper;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Wrapper {
    node: ref<Node, managed, mutable>;
}

type Node {
    wrapper: Wrapper;
}
/// @layout.struct name=Wrapper size=8 align=8
/// @layout.field owner=Wrapper index=0 name=node offset=0 size=8 align=8
/// @layout.struct name=Node size=8 align=8
/// @layout.field owner=Node index=0 name=wrapper offset=0 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_class_construction_methods_and_field_reads() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(start: int32) {
        this.count = start;
    }

    bump(): int32 {
        this.count = this.count + 1;
        return this.count;
    }
}

function tally(start: int32): int32 {
    let counter = new Counter(start);
    return counter.bump() + counter.count;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    count: int32;
}

function test.main.Counter.constructor(v0: ref<Counter, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<Counter, borrowed, exclusive>, v1: int32):
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    store v2, v1
    return
}

function test.main.Counter.bump(v0: ref<Counter, managed, mutable>): int32 {
entry(v0: ref<Counter, managed, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = load v1
    v3: int32 = 1
    v4: int32 = int.add v2, v3
    v5: ref<int32, borrowed, mutable> = field.address v0, 0
    store v5, v4
    v6: ref<int32, borrowed, mutable> = field.address v0, 0
    v7: int32 = load v6
    return v7
}

function test.main.tally(v0: int32): int32 {
    local l0: ref<Counter, managed, mutable>

entry(v0: int32):
    v1: ref<Counter, managed, mutable> = new.zeroed Counter
    v2: ref<Counter, borrowed, exclusive> = cast.bit v1 -> ref<Counter, borrowed, exclusive>
    call test.main.Counter.constructor(v2, v0): (ref<Counter, borrowed, exclusive>, int32) => void
    local.set l0, v1
    v3: ref<Counter, managed, mutable> = local.get l0
    v4: int32 = call test.main.Counter.bump(v3): (ref<Counter, managed, mutable>) => int32
    v5: ref<Counter, managed, mutable> = local.get l0
    v6: ref<int32, borrowed, mutable> = field.address v5, 0
    v7: int32 = load v6
    v8: int32 = int.add v4, v7
    return v8
}
/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_field_reads_across_reference_forms() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(start: int32) {
        this.count = start;
    }
}

function own(counter: ^Counter): int32 {
    return counter.count;
}

function peek(counter: readonly Counter): int32 {
    return counter.count;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    count: int32;
}

function test.main.Counter.constructor(v0: ref<Counter, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<Counter, borrowed, exclusive>, v1: int32):
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    store v2, v1
    return
}

function test.main.own(v0: Counter): int32 {
entry(v0: Counter):
    v1: int32 = field.get v0, 0
    return v1
}

function test.main.peek(v0: ref<Counter, managed, readonly>): int32 {
entry(v0: ref<Counter, managed, readonly>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    return v2
}
/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_nullable_class_union_to_a_niched_reference() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(start: int32) {
        this.count = start;
    }
}

function read(counter: Counter | null): int32 {
    if (counter === null) {
        return 0;
    }
    return counter.count;
}

function forget(counter: Counter): Counter | null {
    if (counter.count > 10) {
        return null;
    }
    return counter;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    count: int32;
}

function test.main.Counter.constructor(v0: ref<Counter, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<Counter, borrowed, exclusive>, v1: int32):
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    store v2, v1
    return
}

function test.main.read(v0: ref<Counter, managed, mutable, nullable>): int32 {
entry(v0: ref<Counter, managed, mutable, nullable>):
    v1: ref<Counter, managed, mutable, nullable> = null
    v2: boolean = int.eq v0, v1
    branch v2, b1, b2

b1:
    v3: int32 = 0
    return v3

b2:
    v4: ref<int32, borrowed, mutable> = field.address v0, 0
    v5: int32 = load v4
    return v5
}

function test.main.forget(v0: ref<Counter, managed, mutable>): ref<Counter, managed, mutable, nullable> {
entry(v0: ref<Counter, managed, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = load v1
    v3: int32 = 10
    v4: boolean = int.gt.s v2, v3
    branch v4, b1, b2

b1:
    v5: ref<Counter, managed, mutable, nullable> = null
    return v5

b2:
    return v0
}
/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_nullish_comparisons_to_null_and_undefined_constants() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(start: int32) {
        this.count = start;
    }
}

function lookup(counter: Counter | undefined): int32 {
    if (counter === undefined) {
        return -1;
    }
    return counter.count;
}

function classify(counter: Counter | undefined | null): int32 {
    if (counter === null) {
        return 0;
    }
    if (counter === undefined) {
        return 1;
    }
    return counter.count;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    count: int32;
}

function test.main.Counter.constructor(v0: ref<Counter, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<Counter, borrowed, exclusive>, v1: int32):
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    store v2, v1
    return
}

function test.main.lookup(v0: ref<Counter, managed, mutable, undefined>): int32 {
entry(v0: ref<Counter, managed, mutable, undefined>):
    v1: ref<Counter, managed, mutable, undefined> = undefined
    v2: boolean = int.eq v0, v1
    branch v2, b1, b2

b1:
    v3: int32 = -1
    return v3

b2:
    v4: ref<int32, borrowed, mutable> = field.address v0, 0
    v5: int32 = load v4
    return v5
}

function test.main.classify(v0: ref<Counter, managed, mutable, nullish>): int32 {
entry(v0: ref<Counter, managed, mutable, nullish>):
    v1: ref<Counter, managed, mutable, nullish> = null
    v2: boolean = int.eq v0, v1
    branch v2, b1, b2

b1:
    v3: int32 = 0
    return v3

b2:
    v4: ref<Counter, managed, mutable, nullish> = undefined
    v5: boolean = int.eq v0, v4
    branch v5, b3, b4

b3:
    v6: int32 = 1
    return v6

b4:
    v7: ref<int32, borrowed, mutable> = field.address v0, 0
    v8: int32 = load v7
    return v8
}
/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_nullable_reference_fields_at_pointer_size() {
    let session = TestSession::single(
        r#"
class Chain {
    next: Chain | null;
    weight: int32;

    constructor(weight: int32) {
        this.next = null;
        this.weight = weight;
    }
}

function total(chain: Chain): int32 {
    return chain.weight;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Chain {
    next: ref<Chain, managed, mutable, nullable>;
    weight: int32;
}

function test.main.Chain.constructor(v0: ref<Chain, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<Chain, borrowed, exclusive>, v1: int32):
    v2: ref<Chain, managed, mutable, nullable> = null
    v3: ref<ref<Chain, managed, mutable, nullable>, borrowed, mutable> = field.address v0, 0
    store v3, v2
    v4: ref<int32, borrowed, mutable> = field.address v0, 1
    store v4, v1
    return
}

function test.main.total(v0: ref<Chain, managed, mutable>): int32 {
entry(v0: ref<Chain, managed, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 1
    v2: int32 = load v1
    return v2
}
/// @layout.struct name=Chain size=16 align=8
/// @layout.field owner=Chain index=0 name=next offset=0 size=8 align=8
/// @layout.field owner=Chain index=1 name=weight offset=8 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_implicit_borrows_at_call_arguments() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(start: int32) {
        this.count = start;
    }
}

function peek(counter: &readonly Counter): int32 {
    return counter.count;
}

function main(counter: Counter): int32 {
    return peek(counter);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    count: int32;
}

function test.main.Counter.constructor(v0: ref<Counter, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<Counter, borrowed, exclusive>, v1: int32):
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    store v2, v1
    return
}

function test.main.peek<'a>(v0: ref<Counter, borrowed, 'a, readonly>): int32 {
entry(v0: ref<Counter, borrowed, 'a, readonly>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function test.main.main(v0: ref<Counter, managed, mutable>): int32 {
entry(v0: ref<Counter, managed, mutable>):
    v1: ref<Counter, borrowed, readonly> = cast.bit v0 -> ref<Counter, borrowed, readonly>
    v2: int32 = call test.main.peek(v1): <'a>(ref<Counter, borrowed, 'a, readonly>) => int32
    return v2
}
/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_owned_construction_into_local_storage() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(start: int32) {
        this.count = start;
    }
}

function own(start: int32): int32 {
    let counter: ^Counter = new Counter(start);
    return counter.count;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    count: int32;
}

function test.main.Counter.constructor(v0: ref<Counter, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<Counter, borrowed, exclusive>, v1: int32):
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    store v2, v1
    return
}

function test.main.own(v0: int32): int32 {
    local l0: Counter
    local l1: Counter

entry(v0: int32):
    v1: ref<Counter, borrowed, exclusive> = local.address l0
    call test.main.Counter.constructor(v1, v0): (ref<Counter, borrowed, exclusive>, int32) => void
    v2: Counter = local.get l0
    local.set l1, v2
    v3: Counter = local.get l1
    v4: int32 = field.get v3, 0
    return v4
}
/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}
