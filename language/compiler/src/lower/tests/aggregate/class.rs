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

    constructor(wrapper: Wrapper) {
        this.wrapper = wrapper;
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Node {
    wrapper: Wrapper;
}

@copy
type Wrapper {
    node: ref<Node, managed, mutable, local>;
}

function test.main.Node.constructor(v0: ref<uninit<Node>, borrowed, exclusive, local>, v1: Wrapper): void {
entry(v0: ref<uninit<Node>, borrowed, exclusive, local>, v1: Wrapper):
    v2: ref<uninit<Wrapper>, borrowed, mutable, local> = field.address v0, 0
    store v2, v1
    return
}

/// @layout.struct name=Node size=8 align=8
/// @layout.field owner=Node index=0 name=wrapper offset=0 size=8 align=8
/// @layout.struct name=Wrapper size=8 align=8
/// @layout.field owner=Wrapper index=0 name=node offset=0 size=8 align=8
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

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32):
    v2: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.Counter.bump(v0: ref<Counter, managed, mutable, local>): int32 {
entry(v0: ref<Counter, managed, mutable, local>):
    v1: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v2: int32 = load v1
    v3: int32 = 1
    v4: int32 = add v2, v3
    v5: ref<int32, borrowed, mutable, local> = field.address v0, 0
    store v5, v4
    v6: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v7: int32 = load v6
    return v7
}

function test.main.tally(v0: int32): int32 {
    local l0: ref<Counter, managed, mutable, local>

entry(v0: int32):
    v1: ref<Counter, managed, mutable, local> = new.zeroed Counter
    v2: ref<uninit<Counter>, borrowed, exclusive, local> = cast.bit v1 -> ref<uninit<Counter>, borrowed, exclusive, local>
    call test.main.Counter.constructor(v2, v0): (ref<uninit<Counter>, borrowed, exclusive, local>, int32) => void
    local.set l0, v1
    v3: ref<Counter, managed, mutable, local> = local.get l0
    v4: int32 = call test.main.Counter.bump(v3): (ref<Counter, managed, mutable, local>) => int32
    v5: ref<Counter, managed, mutable, local> = local.get l0
    v6: ref<int32, borrowed, mutable, local> = field.address v5, 0
    v7: int32 = load v6
    v8: int32 = add v4, v7
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

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32):
    v2: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.own(v0: Counter): int32 {
entry(v0: Counter):
    v1: int32 = field.get v0, 0
    return v1
}

function test.main.peek(v0: ref<Counter, managed, readonly, local>): int32 {
entry(v0: ref<Counter, managed, readonly, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
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

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32):
    v2: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.read(v0: ref<Counter, managed, mutable, nullable, local>): int32 {
entry(v0: ref<Counter, managed, mutable, nullable, local>):
    v1: ref<Counter, managed, mutable, nullable, local> = null
    v2: boolean = eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: int32 = 0
    return v3

b2:
    v4: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v5: int32 = load v4
    return v5
}

function test.main.forget(v0: ref<Counter, managed, mutable, local>): ref<Counter, managed, mutable, nullable, local> {
entry(v0: ref<Counter, managed, mutable, local>):
    v1: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v2: int32 = load v1
    v3: int32 = 10
    v4: boolean = gt v2, v3
    branch v4 => b1 | b2

b1:
    v5: ref<Counter, managed, mutable, nullable, local> = null
    return v5

b2:
    v6: ref<Counter, managed, mutable, nullable, local> = cast.bit v0 -> ref<Counter, managed, mutable, nullable, local>
    return v6
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

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32):
    v2: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.lookup(v0: ref<Counter, managed, mutable, undefined, local>): int32 {
entry(v0: ref<Counter, managed, mutable, undefined, local>):
    v1: ref<Counter, managed, mutable, undefined, local> = undefined
    v2: boolean = eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: int32 = -1
    return v3

b2:
    v4: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v5: int32 = load v4
    return v5
}

function test.main.classify(v0: ref<Counter, managed, mutable, nullish, local>): int32 {
entry(v0: ref<Counter, managed, mutable, nullish, local>):
    v1: ref<Counter, managed, mutable, nullish, local> = null
    v2: boolean = eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: int32 = 0
    return v3

b2:
    v4: ref<Counter, managed, mutable, nullish, local> = undefined
    v5: boolean = eq v0, v4
    branch v5 => b3 | b4

b3:
    v6: int32 = 1
    return v6

b4:
    v7: ref<int32, borrowed, mutable, local> = field.address v0, 0
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
    next: ref<Chain, managed, mutable, nullable, local>;
    weight: int32;
}

function test.main.Chain.constructor(v0: ref<uninit<Chain>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<Chain>, borrowed, exclusive, local>, v1: int32):
    v2: ref<Chain, managed, mutable, nullable, local> = null
    v3: ref<uninit<ref<Chain, managed, mutable, nullable, local>>, borrowed, mutable, local> = field.address v0, 0
    store v3, v2
    v4: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 1
    store v4, v1
    return
}

function test.main.total(v0: ref<Chain, managed, mutable, local>): int32 {
entry(v0: ref<Chain, managed, mutable, local>):
    v1: ref<int32, borrowed, mutable, local> = field.address v0, 1
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

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32):
    v2: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.peek<'a>(v0: ref<Counter, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<Counter, borrowed, 'a, readonly, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function test.main.main(v0: ref<Counter, managed, mutable, local>): int32 {
entry(v0: ref<Counter, managed, mutable, local>):
    v1: ref<Counter, borrowed, 'frame, readonly, local> = cast.bit v0 -> ref<Counter, borrowed, 'frame, readonly, local>
    v2: int32 = call test.main.peek(v1): <'a>(ref<Counter, borrowed, 'a, readonly, local>) => int32
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

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, local>, v1: int32):
    v2: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.own(v0: int32): int32 {
    local l0: Counter
    local l1: Counter

entry(v0: int32):
    v1: ref<Counter, borrowed, exclusive, frame> = local.address l0
    v2: ref<uninit<Counter>, borrowed, exclusive, frame> = cast.bit v1 -> ref<uninit<Counter>, borrowed, exclusive, frame>
    call test.main.Counter.constructor(v2, v0): (ref<uninit<Counter>, borrowed, exclusive, local>, int32) => void
    v3: Counter = local.get l0
    local.set l1, v3
    v4: Counter = local.get l1
    v5: int32 = field.get v4, 0
    return v5
}

/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_default_construction_synthesizes_an_initializer_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 3;
}

function make(): ^Counter {
    return new Counter();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    count: int32;
}

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, local>):
    v1: int32 = 3
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.make(): Counter {
    local l0: Counter

entry:
    v0: ref<Counter, borrowed, exclusive, frame> = local.address l0
    v1: ref<uninit<Counter>, borrowed, exclusive, frame> = cast.bit v0 -> ref<uninit<Counter>, borrowed, exclusive, frame>
    call test.main.Counter.constructor(v1): (ref<uninit<Counter>, borrowed, exclusive, local>) => void
    v2: Counter = local.get l0
    return v2
}

/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_store_derived_initializers_after_the_super_call() {
    let session = TestSession::single(
        r#"
class Base {
    tag: int32 = 1;

    constructor() {}
}

class Derived extends Base {
    extra: int32 = 2;

    constructor() {
        super();
    }
}

function build(): ^Derived {
    return new Derived();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Base {
    tag: int32;
}

type Derived {
    tag: int32;
    extra: int32;
}

function test.main.Base.constructor(v0: ref<uninit<Base>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Base>, borrowed, exclusive, local>):
    v1: int32 = 1
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.Derived.constructor(v0: ref<uninit<Derived>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Derived>, borrowed, exclusive, local>):
    v1: ref<uninit<Base>, borrowed, exclusive, local> = cast.bit v0 -> ref<uninit<Base>, borrowed, exclusive, local>
    call test.main.Base.constructor(v1): (ref<uninit<Base>, borrowed, exclusive, local>) => void
    v2: int32 = 2
    v3: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 1
    store v3, v2
    v4: void = undefined
    return
}

function test.main.build(): Derived {
    local l0: Derived

entry:
    v0: ref<Derived, borrowed, exclusive, frame> = local.address l0
    v1: ref<uninit<Derived>, borrowed, exclusive, frame> = cast.bit v0 -> ref<uninit<Derived>, borrowed, exclusive, frame>
    call test.main.Derived.constructor(v1): (ref<uninit<Derived>, borrowed, exclusive, local>) => void
    v2: Derived = local.get l0
    return v2
}

/// @layout.struct name=Base size=4 align=4
/// @layout.field owner=Base index=0 name=tag offset=0 size=4 align=4
/// @layout.struct name=Derived size=8 align=4
/// @layout.field owner=Derived index=0 name=tag offset=0 size=4 align=4
/// @layout.field owner=Derived index=1 name=extra offset=4 size=4 align=4
"#,
    );
}
