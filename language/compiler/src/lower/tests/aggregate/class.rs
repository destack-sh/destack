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

    session.assert_mir_function("main.tspp", "test.main.Node.constructor", r#"
type test.main.Wrapper {
    node: ref<test.main.Node, managed, mutable, local>;
}

@nocopy
type test.main.Node {
    wrapper: test.main.Wrapper;
}

constructor test.main.Node.constructor(v0: ref<uninit<test.main.Node>, borrowed, 'managed, mutable>, v1: test.main.Wrapper): void {
    local l0: test.main.Wrapper
    local l1: ref<uninit<test.main.Node>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Node>, borrowed, 'managed, mutable>, v1: test.main.Wrapper):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Node>, borrowed, 'managed, mutable> = load l1
    v3: test.main.Wrapper = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Wrapper size=8 align=8
/// @layout.field owner=test.main.Wrapper index=0 name=node offset=0 size=8 align=8
/// @layout.struct name=test.main.Node size=8 align=8
/// @layout.field owner=test.main.Node index=0 name=wrapper offset=0 size=8 align=8
"#);
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

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.Counter.bump",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.Counter.bump(v0: ref<test.main.Counter, managed, mutable, local>): int32 {
    local l0: ref<test.main.Counter, managed, mutable, local>

entry(v0: ref<test.main.Counter, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Counter, managed, mutable, local> = load l0
    v2: ref<test.main.Counter, managed, mutable, local> = load l0
    v3: int32 = load (*v2).0
    v4: int32 = 1
    v5: int32 = add v3, v4
    store (*v1).0, v5
    v6: ref<test.main.Counter, managed, mutable, local> = load l0
    v7: int32 = load (*v6).0
    return v7
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.tally",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.tally(v0: int32): int32 {
    local l0: int32
    local l1: ref<test.main.Counter, managed, mutable, local>

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v2 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v3, v1): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, int32) => void
    store l1, v2
    v4: ref<test.main.Counter, managed, mutable, local> = load l1
    v5: int32 = call test.main.Counter.bump(v4): (ref<test.main.Counter, managed, mutable, local>) => int32
    v6: ref<test.main.Counter, managed, mutable, local> = load l1
    v7: int32 = load (*v6).0
    v8: int32 = add v5, v7
    return v8
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
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

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.own",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.own(v0: test.main.Counter): int32 {
    local l0: test.main.Counter

entry(v0: test.main.Counter):
    store l0, v0
    v1: int32 = load (l0).0
    return v1
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.peek",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.peek(v0: ref<test.main.Counter, managed, readonly, local>): int32 {
    local l0: ref<test.main.Counter, managed, readonly, local>

entry(v0: ref<test.main.Counter, managed, readonly, local>):
    store l0, v0
    v1: ref<test.main.Counter, managed, readonly, local> = load l0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
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

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.read(v0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; }): int32 {
    local l0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; }

entry(v0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; }):
    store l0, v0
    v1: uint1 = variant.tag.load l0
    v2: uint1 = 1
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: int32 = 0
    return v4

b2:
    v5: ref<test.main.Counter, borrowed, 'managed, mutable> = address (*(l0 as 0))
    v6: int32 = load (*v5).0
    return v6
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.forget",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.forget(v0: ref<test.main.Counter, managed, mutable, local>): variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; } {
    local l0: ref<test.main.Counter, managed, mutable, local>

entry(v0: ref<test.main.Counter, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Counter, managed, mutable, local> = load l0
    v2: int32 = load (*v1).0
    v3: int32 = 10
    v4: boolean = gt v2, v3
    branch v4 => b1 | b2

b1:
    v5: null = zeroed
    v6: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; } = variant.new 1
    return v6

b2:
    v7: ref<test.main.Counter, managed, mutable, local> = load l0
    v8: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; } = variant.new 0, v7
    return v8
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
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

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.lookup",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.lookup(v0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; }): int32 {
    local l0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; }):
    store l0, v0
    v1: uint1 = variant.tag.load l0
    v2: uint1 = 1
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: int32 = -1
    return v4

b2:
    v5: ref<test.main.Counter, borrowed, 'managed, mutable> = address (*(l0 as 0))
    v6: int32 = load (*v5).0
    return v6
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@9 size=8 align=8
/// @layout.discriminant owner=type@9 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@9 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@9 index=1 discriminant=1 payload_offset=0
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.classify",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.classify(v0: variant<uint2> { 0uint2 = ref<test.main.Counter, managed, mutable, local>; 1uint2 = void; 2uint2 = null; }): int32 {
    local l0: variant<uint2> { 0uint2 = ref<test.main.Counter, managed, mutable, local>; 1uint2 = void; 2uint2 = null; }

entry(v0: variant<uint2> { 0uint2 = ref<test.main.Counter, managed, mutable, local>; 1uint2 = void; 2uint2 = null; }):
    store l0, v0
    v1: uint2 = variant.tag.load l0
    v2: uint2 = 2
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: int32 = 0
    return v4

b2:
    v5: uint1 = variant.tag.load l0
    v6: uint1 = 1
    v7: boolean = eq v5, v6
    branch v7 => b3 | b4

b3:
    v8: int32 = 1
    return v8

b4:
    v9: ref<test.main.Counter, borrowed, 'managed, mutable> = address (*(l0 as 0))
    v10: int32 = load (*v9).0
    return v10
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@12 size=8 align=8
/// @layout.discriminant owner=type@12 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@12 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@12 index=1 discriminant=1 payload_offset=0
/// @layout.case owner=type@12 index=2 discriminant=2 payload_offset=0
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

    session.assert_mir_function("main.tspp", "test.main.Chain.constructor", r#"
@nocopy
type test.main.Chain {
    next: variant<uint1> { 0uint1 = ref<test.main.Chain, managed, mutable, local>; 1uint1 = null; };
    weight: int32;
}

constructor test.main.Chain.constructor(v0: ref<uninit<test.main.Chain>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Chain>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Chain>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Chain>, borrowed, 'managed, mutable> = load l1
    v3: null = zeroed
    v4: variant<uint1> { 0uint1 = ref<test.main.Chain, managed, mutable, local>; 1uint1 = null; } = variant.new 1
    store (*v2).0, v4
    v5: ref<uninit<test.main.Chain>, borrowed, 'managed, mutable> = load l1
    v6: int32 = load l0
    store (*v5).1, v6
    return
}

/// @layout.struct name=test.main.Chain size=16 align=8
/// @layout.field owner=test.main.Chain index=0 name=next offset=0 size=8 align=8
/// @layout.field owner=test.main.Chain index=1 name=weight offset=8 size=4 align=4
/// @layout.variant name=type@4 size=8 align=8
/// @layout.discriminant owner=type@4 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@4 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@4 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.total",
        r#"
@nocopy
type test.main.Chain {
    next: variant<uint1> { 0uint1 = ref<test.main.Chain, managed, mutable, local>; 1uint1 = null; };
    weight: int32;
}

function test.main.total(v0: ref<test.main.Chain, managed, mutable, local>): int32 {
    local l0: ref<test.main.Chain, managed, mutable, local>

entry(v0: ref<test.main.Chain, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Chain, managed, mutable, local> = load l0
    v2: int32 = load (*v1).1
    return v2
}

/// @layout.struct name=test.main.Chain size=16 align=8
/// @layout.field owner=test.main.Chain index=0 name=next offset=0 size=8 align=8
/// @layout.field owner=test.main.Chain index=1 name=weight offset=8 size=4 align=4
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

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.peek",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.peek<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.main",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.main(v0: ref<test.main.Counter, managed, mutable, local>): int32 {
    local l0: ref<test.main.Counter, managed, mutable, local>

entry(v0: ref<test.main.Counter, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Counter, managed, mutable, local> = load l0
    v2: ref<test.main.Counter, borrowed, 'managed, readonly> = cast.bit v1 -> ref<test.main.Counter, borrowed, 'managed, readonly>
    v3: int32 = call test.main.peek(v2): (ref<test.main.Counter, borrowed, 'managed, readonly>) => int32
    return v3
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_owned_construction_into_local_storage() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(&exclusive this, start: int32) {
        this.count = start;
    }
}

function own(start: int32): int32 {
    let counter: ^Counter = new Counter(start);
    return counter.count;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive> = address (*l1)
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.own", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.own(v0: int32): int32 {
    local l0: int32
    local l1: test.main.Counter
    local l2: test.main.Counter

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: ref<test.main.Counter, borrowed, 'frame, exclusive> = address l1
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, exclusive> = cast.bit v2 -> ref<uninit<test.main.Counter>, borrowed, 'managed, exclusive>
    call test.main.Counter.constructor(v3, v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>, int32) => void
    v4: test.main.Counter = load l1
    store l2, v4
    v5: int32 = load (l2).0
    return v5
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.Counter.constructor",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 3
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.make",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.make(): test.main.Counter {
    local l0: test.main.Counter

entry:
    v0: ref<test.main.Counter, borrowed, 'frame, exclusive> = address l0
    v1: ref<uninit<test.main.Counter>, borrowed, 'frame, exclusive> = cast.bit v0 -> ref<uninit<test.main.Counter>, borrowed, 'frame, exclusive>
    call test.main.Counter.constructor(v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>) => void
    v2: test.main.Counter = load l0
    return v2
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_store_derived_initializers_after_the_super_call() {
    let session = TestSession::single(
        r#"
class Base {
    tag: int32 = 1;

    constructor(&exclusive this) {}
}

class Derived extends Base {
    extra: int32 = 2;

    constructor(&exclusive this) {
        super();
    }
}

function build(): ^Derived {
    return new Derived();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Base.constructor",
        r#"
@nocopy
type test.main.Base {
    tag: int32;
}

constructor test.main.Base.constructor<'a>(v0: ref<uninit<test.main.Base>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Base>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Base>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 1
    v2: ref<uninit<test.main.Base>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Base size=4 align=4
/// @layout.field owner=test.main.Base index=0 name=tag offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Derived.constructor", r#"
@nocopy
type test.main.Base {
    tag: int32;
}

@nocopy
type test.main.Derived {
    tag: int32;
    extra: int32;
}

constructor test.main.Derived.constructor<'a>(v0: ref<uninit<test.main.Derived>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Derived>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Derived>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: ref<uninit<test.main.Derived>, borrowed, 'a, exclusive> = address (*l0)
    v2: ref<uninit<test.main.Base>, borrowed, 'managed, exclusive> = cast.bit v1 -> ref<uninit<test.main.Base>, borrowed, 'managed, exclusive>
    call test.main.Base.constructor(v2): <'a_1>(ref<uninit<test.main.Base>, borrowed, 'a_1, exclusive>) => void
    v3: int32 = 2
    v4: ref<uninit<test.main.Derived>, borrowed, 'a, exclusive> = address (*l0)
    store (*v4).1, v3
    v5: void = zeroed
    return
}

/// @layout.struct name=test.main.Base size=4 align=4
/// @layout.field owner=test.main.Base index=0 name=tag offset=0 size=4 align=4
/// @layout.struct name=test.main.Derived size=8 align=4
/// @layout.field owner=test.main.Derived index=0 name=tag offset=0 size=4 align=4
/// @layout.field owner=test.main.Derived index=1 name=extra offset=4 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.build",
        r#"
@nocopy
type test.main.Derived {
    tag: int32;
    extra: int32;
}

function test.main.build(): test.main.Derived {
    local l0: test.main.Derived

entry:
    v0: ref<test.main.Derived, borrowed, 'frame, exclusive> = address l0
    v1: ref<uninit<test.main.Derived>, borrowed, 'managed, exclusive> = cast.bit v0 -> ref<uninit<test.main.Derived>, borrowed, 'managed, exclusive>
    call test.main.Derived.constructor(v1): <'a>(ref<uninit<test.main.Derived>, borrowed, 'a, exclusive>) => void
    v2: test.main.Derived = load l0
    return v2
}

/// @layout.struct name=test.main.Derived size=8 align=4
/// @layout.field owner=test.main.Derived index=0 name=tag offset=0 size=4 align=4
/// @layout.field owner=test.main.Derived index=1 name=extra offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_a_handle_field_borrow_through_a_class_receiver() {
    let session = TestSession::single(
        r#"
export class Failure {
    readonly message: string;

    constructor(message: string) {
        this.message = message;
    }

    display(): &readonly string {
        return this.message;
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Failure.display",
        r#"
@nocopy
type test.main.Failure {
    message: ref<String, managed, mutable, local>;
}

@nocopy
@languageItem("string.String")
type String;

function test.main.Failure.display<'a>(v0: ref<test.main.Failure, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Failure, borrowed, 'a, readonly>

entry(v0: ref<test.main.Failure, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Failure, borrowed, 'a, readonly> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v1).0
    v3: ref<String, managed, mutable, local> = load (*v2)
    v4: ref<String, borrowed, 'a, readonly> = cast.bit v3 -> ref<String, borrowed, 'a, readonly>
    return v4
}

/// @layout.struct name=test.main.Failure size=8 align=8
/// @layout.field owner=test.main.Failure index=0 name=message offset=0 size=8 align=8
"#,
    );
    session.assert_mir_function("main.tspp", "test.main.Failure.constructor", r#"
@nocopy
type test.main.Failure {
    message: ref<String, managed, mutable, local>;
}

@nocopy
@languageItem("string.String")
type String;

constructor test.main.Failure.constructor(v0: ref<uninit<test.main.Failure>, borrowed, 'managed, mutable>, v1: ref<String, managed, mutable, local>): void {
    local l0: ref<String, managed, mutable, local>
    local l1: ref<uninit<test.main.Failure>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Failure>, borrowed, 'managed, mutable>, v1: ref<String, managed, mutable, local>):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Failure>, borrowed, 'managed, mutable> = load l1
    v3: ref<String, managed, mutable, local> = load l0
    store (*v2).0, v3
    return
}

/// @layout.struct name=test.main.Failure size=8 align=8
/// @layout.field owner=test.main.Failure index=0 name=message offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.tspp", "test.main.Failure.display", r#"
@nocopy
type test.main.Failure {
    message: ref<String, managed, mutable, local>;
}

@nocopy
@languageItem("string.String")
type String;

function test.main.Failure.display<'a>(v0: ref<test.main.Failure, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<test.main.Failure, borrowed, 'a, readonly>

entry(v0: ref<test.main.Failure, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Failure, borrowed, 'a, readonly> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v1).0
    v3: ref<String, managed, mutable, local> = load (*v2)
    v4: ref<String, borrowed, 'a, readonly> = cast.bit v3 -> ref<String, borrowed, 'a, readonly>
    return v4
}

/// @layout.struct name=test.main.Failure size=8 align=8
/// @layout.field owner=test.main.Failure index=0 name=message offset=0 size=8 align=8
"#);
}

/// An optional field a constructor leaves unassigned stores undefined before the body runs.
#[test]
fn test_store_undefined_at_an_unassigned_optional_field() {
    let session = TestSession::single(
        r#"
class Runtime {
    id: int32;
    name?: string;

    constructor(id: int32) {
        this.id = id;
    }
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Runtime.constructor", r#"
@nocopy
type test.main.Runtime {
    id: int32;
    name: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

constructor test.main.Runtime.constructor(v0: ref<uninit<test.main.Runtime>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Runtime>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Runtime>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Runtime>, borrowed, 'managed, mutable> = address (*l1)
    v3: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 1
    store (*v2).1, v3
    v4: ref<uninit<test.main.Runtime>, borrowed, 'managed, mutable> = load l1
    v5: int32 = load l0
    store (*v4).0, v5
    return
}

/// @layout.struct name=test.main.Runtime size=16 align=8
/// @layout.field owner=test.main.Runtime index=0 name=id offset=8 size=4 align=4
/// @layout.field owner=test.main.Runtime index=1 name=name offset=0 size=8 align=8
/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
"#);
}

/// A field default the constructor body assigns is left to that assignment.
#[test]
fn test_skip_a_field_default_the_constructor_assigns() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 1;

    constructor() {
        this.count = 2;
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Counter.constructor",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>):
    store l0, v0
    v1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l0
    v2: int32 = 2
    store (*v1).0, v2
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_borrow_of_an_owned_slice_field_through_its_place() {
    let session = TestSession::single(
        r#"
import { Slice } from "tspp:collections";

class Holder {
    storage: ^[int32] = Slice.new();
}

function size(holder: &readonly Holder): isize {
    let view: &readonly [int32] = &readonly holder.storage;

    return view.size;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Holder.constructor", r#"
@nocopy
type test.main.Holder {
    storage: slice<int32, unique, mutable>;
}

constructor test.main.Holder.constructor<'a>(v0: ref<uninit<test.main.Holder>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Holder>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Holder>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: slice<int32, unique, mutable> = call Slice.new<int32>(): () => slice<int32, unique, mutable>
    v2: ref<uninit<test.main.Holder>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.Holder size=16 align=8
/// @layout.field owner=test.main.Holder index=0 name=storage offset=0 size=16 align=8
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.size",
        r#"
@nocopy
type test.main.Holder {
    storage: slice<int32, unique, mutable>;
}

function test.main.size<'a>(v0: ref<test.main.Holder, borrowed, 'a, readonly>): isize {
    local l0: ref<test.main.Holder, borrowed, 'a, readonly>
    local l1: slice<int32, borrowed, 'a, readonly>

entry(v0: ref<test.main.Holder, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Holder, borrowed, 'a, readonly> = load l0
    v2: slice<int32, borrowed, 'a, readonly> = address (*(*v1).0)
    store l1, v2
    v3: slice<int32, borrowed, 'a, readonly> = load l1
    v4: isize = call Slice.size.get<int32>(v3): (slice<int32, borrowed, 'a, readonly>) => isize
    return v4
}

/// @layout.struct name=test.main.Holder size=16 align=8
/// @layout.field owner=test.main.Holder index=0 name=storage offset=0 size=16 align=8
"#,
    );
}

#[test]
fn test_lower_a_conditionally_assigned_field_over_its_default() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 1;

    constructor(reset: boolean) {
        if (reset) {
            this.count = 0;
        }
    }
}

function make(reset: boolean): Counter {
    return new Counter(reset);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: boolean): void {
    local l0: boolean
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: boolean):
    store l0, v1
    store l1, v0
    v2: int32 = 1
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = address (*l1)
    store (*v3).0, v2
    v4: boolean = load l0
    branch v4 => b1 | b2

b1:
    v5: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v6: int32 = 0
    store (*v5).0, v6
    jump b2

b2:
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.make", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.make(v0: boolean): ref<test.main.Counter, managed, mutable, local> {
    local l0: boolean

entry(v0: boolean):
    store l0, v0
    v1: boolean = load l0
    v2: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v2 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v3, v1): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, boolean) => void
    return v2
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);
}

#[test]
fn test_lower_a_compound_assigned_field_over_its_default() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 1;

    constructor() {
        this.count += 1;
    }
}

function make(): Counter {
    return new Counter();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Counter.constructor",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>):
    store l0, v0
    v1: int32 = 1
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = address (*l0)
    store (*v2).0, v1
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l0
    v4: int32 = load (*v3).0
    v5: int32 = 1
    v6: int32 = add v4, v5
    store (*v3).0, v6
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.make",
        r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.make(): ref<test.main.Counter, managed, mutable, local> {
entry:
    v0: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v0 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v1): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>) => void
    return v0
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}

/// Every form of a class object and a struct lowers to one word or the value.
#[test]
fn test_lower_every_form_of_a_class_object_and_a_struct() {
    let session = TestSession::single(
        r#"
class User {
    name: int32 = 0;
}

struct Point {
    x: int32;
}

function forms(
    managed: User,
    viewed: readonly User,
    owned: ^User,
    readable: &readonly User,
    mutable: &User,
    frozen: &immutable User,
    exclusive: &exclusive User,
    point: Point,
    ownedPoint: ^Point,
    viewedPoint: readonly Point,
    exclusivePoint: &exclusive Point,
): void {}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.forms", r#"
@nocopy
type test.main.User {
    name: int32;
}

type test.main.Point {
    x: int32;
}

function test.main.forms<'a, 'b, 'c, 'd, 'e>(v0: ref<test.main.User, managed, mutable, local>, v1: ref<test.main.User, managed, readonly, local>, v2: test.main.User, v3: ref<test.main.User, borrowed, 'a, readonly>, v4: ref<test.main.User, borrowed, 'b, mutable>, v5: ref<test.main.User, borrowed, 'c, immutable>, v6: ref<test.main.User, borrowed, 'd, exclusive>, v7: test.main.Point, v8: test.main.Point, v9: test.main.Point, v10: ref<test.main.Point, borrowed, 'e, exclusive>): void {
    local l0: ref<test.main.User, managed, mutable, local>
    local l1: ref<test.main.User, managed, readonly, local>
    local l2: test.main.User
    local l3: ref<test.main.User, borrowed, 'a, readonly>
    local l4: ref<test.main.User, borrowed, 'b, mutable>
    local l5: ref<test.main.User, borrowed, 'c, immutable>
    local l6: ref<test.main.User, borrowed, 'd, exclusive>
    local l7: test.main.Point
    local l8: test.main.Point
    local l9: test.main.Point
    local l10: ref<test.main.Point, borrowed, 'e, exclusive>

entry(v0: ref<test.main.User, managed, mutable, local>, v1: ref<test.main.User, managed, readonly, local>, v2: test.main.User, v3: ref<test.main.User, borrowed, 'a, readonly>, v4: ref<test.main.User, borrowed, 'b, mutable>, v5: ref<test.main.User, borrowed, 'c, immutable>, v6: ref<test.main.User, borrowed, 'd, exclusive>, v7: test.main.Point, v8: test.main.Point, v9: test.main.Point, v10: ref<test.main.Point, borrowed, 'e, exclusive>):
    store l0, v0
    store l1, v1
    store l2, v2
    store l3, v3
    store l4, v4
    store l5, v5
    store l6, v6
    store l7, v7
    store l8, v8
    store l9, v9
    store l10, v10
    return
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=name offset=0 size=4 align=4
/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#);
}

/// A payload borrow of a frame-owned object addresses the payload from the local directly.
#[test]
fn test_lower_a_payload_borrow_of_a_frame_owned_object_from_its_place() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    slot: ^Payload | undefined;

    constructor(&exclusive this, slot: ^Payload | undefined) {
        this.slot = slot;
    }
}

function read(payload: &readonly Payload): int32 {
    return payload.value;
}

export function inspect(slot: ^Payload | undefined): int32 {
    const holder: ^Holder = new Holder(slot);
    if (holder.slot !== undefined) {
        const held = &readonly holder.slot;
        return read(held);
    }
    return 0;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.inspect", r#"
type test.main.Payload {
    value: int32;
}

@nocopy
type test.main.Holder {
    slot: variant<uint1> { 0uint1 = test.main.Payload; 1uint1 = void; };
}

function test.main.inspect(v0: variant<uint1> { 0uint1 = test.main.Payload; 1uint1 = void; }): int32 {
    local l0: variant<uint1> { 0uint1 = test.main.Payload; 1uint1 = void; }
    local l1: test.main.Holder
    local l2: test.main.Holder
    local l3: ref<test.main.Payload, borrowed, 'frame, readonly>

entry(v0: variant<uint1> { 0uint1 = test.main.Payload; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = test.main.Payload; 1uint1 = void; } = load l0
    v2: ref<test.main.Holder, borrowed, 'frame, exclusive> = address l1
    v3: ref<uninit<test.main.Holder>, borrowed, 'managed, exclusive> = cast.bit v2 -> ref<uninit<test.main.Holder>, borrowed, 'managed, exclusive>
    call test.main.Holder.constructor(v3, v1): <'a>(ref<uninit<test.main.Holder>, borrowed, 'a, exclusive>, variant<uint1> { 0uint1 = test.main.Payload; 1uint1 = void; }) => void
    v4: test.main.Holder = load l1
    store l2, v4
    v5: uint1 = variant.tag.load (l2).0
    v6: uint1 = 1
    v7: boolean = eq v5, v6
    v8: boolean = not v7
    branch v8 => b1 | b2

b1:
    v9: ref<test.main.Payload, borrowed, 'frame, readonly> = address ((l2).0 as 0)
    store l3, v9
    v10: ref<test.main.Payload, borrowed, 'frame, readonly> = load l3
    v11: int32 = call test.main.read(v10): (ref<test.main.Payload, borrowed, 'frame, readonly>) => int32
    return v11

b2:
    v12: int32 = 0
    return v12
}

/// @layout.struct name=test.main.Payload size=4 align=4
/// @layout.field owner=test.main.Payload index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=test.main.Holder size=8 align=4
/// @layout.field owner=test.main.Holder index=0 name=slot offset=0 size=8 align=4
/// @layout.variant name=type@7 size=8 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
"#);
}

/// A derived receiver reinterprets at its base class for an inherited method call.
#[test]
fn test_reinterpret_a_derived_receiver_at_its_base_for_an_inherited_call() {
    let session = TestSession::single(
        r#"
class Base {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    read(this): int32 {
        return this.value;
    }
}

class Derived extends Base {
    extra: int32;

    constructor(value: int32, extra: int32) {
        super(value);
        this.extra = extra;
    }
}

function read(derived: Derived): int32 {
    return derived.read();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
@nocopy
type test.main.Base {
    value: int32;
}

@nocopy
type test.main.Derived {
    value: int32;
    extra: int32;
}

function test.main.read(v0: ref<test.main.Derived, managed, mutable, local>): int32 {
    local l0: ref<test.main.Derived, managed, mutable, local>

entry(v0: ref<test.main.Derived, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Derived, managed, mutable, local> = load l0
    v2: ref<test.main.Base, managed, mutable, local> = cast.bit v1 -> ref<test.main.Base, managed, mutable, local>
    v3: int32 = call test.main.Base.read(v2): (ref<test.main.Base, managed, mutable, local>) => int32
    return v3
}

/// @layout.struct name=test.main.Base size=4 align=4
/// @layout.field owner=test.main.Base index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=test.main.Derived size=8 align=4
/// @layout.field owner=test.main.Derived index=0 name=value offset=0 size=4 align=4
/// @layout.field owner=test.main.Derived index=1 name=extra offset=4 size=4 align=4
"#,
    );
}

/// A readonly base method borrows a derived receiver's object at its base.
#[test]
fn test_borrow_a_derived_receiver_readonly_at_its_base_for_an_inherited_call() {
    let session = TestSession::single(
        r#"
class Base {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    read(&readonly this): int32 {
        return this.value;
    }
}

class Derived extends Base {
    extra: int32;

    constructor(value: int32, extra: int32) {
        super(value);
        this.extra = extra;
    }
}

function read(derived: Derived): int32 {
    return derived.read();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
@nocopy
type test.main.Base {
    value: int32;
}

@nocopy
type test.main.Derived {
    value: int32;
    extra: int32;
}

function test.main.read(v0: ref<test.main.Derived, managed, mutable, local>): int32 {
    local l0: ref<test.main.Derived, managed, mutable, local>

entry(v0: ref<test.main.Derived, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Derived, managed, mutable, local> = load l0
    v2: ref<test.main.Derived, borrowed, 'managed, readonly> = cast.bit v1 -> ref<test.main.Derived, borrowed, 'managed, readonly>
    v3: ref<test.main.Base, borrowed, 'managed, readonly> = cast.bit v2 -> ref<test.main.Base, borrowed, 'managed, readonly>
    v4: int32 = call test.main.Base.read(v3): (ref<test.main.Base, borrowed, 'managed, readonly>) => int32
    return v4
}

/// @layout.struct name=test.main.Base size=4 align=4
/// @layout.field owner=test.main.Base index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=test.main.Derived size=8 align=4
/// @layout.field owner=test.main.Derived index=0 name=value offset=0 size=4 align=4
/// @layout.field owner=test.main.Derived index=1 name=extra offset=4 size=4 align=4
"#,
    );
}
