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

    session.assert_mir_function("main.ds", "test.main.Node.constructor", r#"
type test.main.Node {
    wrapper: test.main.Wrapper;
}

@copy
type test.main.Wrapper {
    node: ref<test.main.Node, managed, mutable, local>;
}

function test.main.Node.constructor<'a>(v0: ref<uninit<test.main.Node>, borrowed, 'a, mutable, local>, v1: test.main.Wrapper): void {
    local l0: test.main.Wrapper
    local l1: ref<uninit<test.main.Node>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Node>, borrowed, 'a, mutable, local>, v1: test.main.Wrapper):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Node>, borrowed, 'a, mutable, local> = local.get l1
    v3: test.main.Wrapper = local.get l0
    v4: ref<uninit<test.main.Wrapper>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.Node size=8 align=8
/// @layout.field owner=test.main.Node index=0 name=wrapper offset=0 size=8 align=8
/// @layout.struct name=test.main.Wrapper size=8 align=8
/// @layout.field owner=test.main.Wrapper index=0 name=node offset=0 size=8 align=8
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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.Counter.bump",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.bump(v0: ref<test.main.Counter, managed, mutable, local>): int32 {
    local l0: ref<test.main.Counter, managed, mutable, local>

entry(v0: ref<test.main.Counter, managed, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, managed, mutable, local> = local.get l0
    v2: ref<test.main.Counter, managed, mutable, local> = local.get l0
    v3: ref<int32, borrowed, 'managed, readonly, local> = field.project v2, 0
    v4: int32 = load v3
    v5: int32 = 1
    v6: int32 = add v4, v5
    v7: ref<int32, borrowed, 'managed, mutable, local> = field.project v1, 0
    store v7, v6
    v8: ref<test.main.Counter, managed, mutable, local> = local.get l0
    v9: ref<int32, borrowed, 'managed, readonly, local> = field.project v8, 0
    v10: int32 = load v9
    return v10
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.tally",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.tally(v0: int32): int32 {
    local l0: int32
    local l1: ref<test.main.Counter, managed, mutable, local>

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v2 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v3, v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, int32) => void
    local.set l1, v2
    v4: ref<test.main.Counter, managed, mutable, local> = local.get l1
    v5: int32 = call test.main.Counter.bump(v4): (ref<test.main.Counter, managed, mutable, local>) => int32
    v6: ref<test.main.Counter, managed, mutable, local> = local.get l1
    v7: ref<int32, borrowed, 'managed, readonly, local> = field.project v6, 0
    v8: int32 = load v7
    v9: int32 = add v5, v8
    return v9
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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.own",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.own(v0: test.main.Counter): int32 {
    local l0: test.main.Counter

entry(v0: test.main.Counter):
    local.set l0, v0
    v1: ref<test.main.Counter, borrowed, 'frame, readonly, frame> = local.project l0
    v2: ref<int32, borrowed, 'frame, readonly, frame> = field.project v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.peek",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.peek(v0: ref<test.main.Counter, managed, readonly, local>): int32 {
    local l0: ref<test.main.Counter, managed, readonly, local>

entry(v0: ref<test.main.Counter, managed, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, managed, readonly, local> = local.get l0
    v2: ref<int32, borrowed, 'managed, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    return v3
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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.read(v0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; }): int32 {
    local l0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; }

entry(v0: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; }):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; } = local.get l0
    v2: uint1 = variant.tag v1
    v3: uint1 = 1
    v4: boolean = eq v2, v3
    branch v4 => b1 | b2

b1:
    v5: int32 = 0
    return v5

b2:
    v6: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; } = local.get l0
    v7: ref<test.main.Counter, managed, mutable, local> = variant.payload v6, 0
    v8: ref<int32, borrowed, 'managed, readonly, local> = field.project v7, 0
    v9: int32 = load v8
    return v9
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=8
/// @layout.discriminant owner=type@11 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=0
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.forget",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.forget(v0: ref<test.main.Counter, managed, mutable, local>): variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; } {
    local l0: ref<test.main.Counter, managed, mutable, local>

entry(v0: ref<test.main.Counter, managed, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, managed, mutable, local> = local.get l0
    v2: ref<int32, borrowed, 'managed, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    v4: int32 = 10
    v5: boolean = gt v3, v4
    branch v5 => b1 | b2

b1:
    v6: null = zeroed
    v7: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; } = variant.new 1
    return v7

b2:
    v8: ref<test.main.Counter, managed, mutable, local> = local.get l0
    v9: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = null; } = variant.new 0, v8
    return v9
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=8
/// @layout.discriminant owner=type@11 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=0
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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.lookup",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.lookup(v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; }): int32 {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; }

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; }):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } = local.get l0
    v2: uint1 = variant.tag v1
    v3: uint1 = 0
    v4: boolean = eq v2, v3
    branch v4 => b1 | b2

b1:
    v5: int32 = -1
    return v5

b2:
    v6: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } = local.get l0
    v7: ref<test.main.Counter, managed, mutable, local> = variant.payload v6, 1
    v8: ref<int32, borrowed, 'managed, readonly, local> = field.project v7, 0
    v9: int32 = load v8
    return v9
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.classify",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.classify(v0: variant<uint2> { 0uint2 = void; 1uint2 = ref<test.main.Counter, managed, mutable, local>; 2uint2 = null; }): int32 {
    local l0: variant<uint2> { 0uint2 = void; 1uint2 = ref<test.main.Counter, managed, mutable, local>; 2uint2 = null; }
    local l1: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; }

entry(v0: variant<uint2> { 0uint2 = void; 1uint2 = ref<test.main.Counter, managed, mutable, local>; 2uint2 = null; }):
    local.set l0, v0
    v1: variant<uint2> { 0uint2 = void; 1uint2 = ref<test.main.Counter, managed, mutable, local>; 2uint2 = null; } = local.get l0
    v2: uint2 = variant.tag v1
    v3: uint2 = 2
    v4: boolean = eq v2, v3
    branch v4 => b1 | b2

b1:
    v5: int32 = 0
    return v5

b2:
    v6: variant<uint2> { 0uint2 = void; 1uint2 = ref<test.main.Counter, managed, mutable, local>; 2uint2 = null; } = local.get l0
    variant.switch v6, 1 => b5, 0 => b6, else b4

b3:
    v10: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } = local.get l1
    v11: uint1 = variant.tag v10
    v12: uint1 = 0
    v13: boolean = eq v11, v12
    branch v13 => b7 | b8

b4:
    panic

b5:
    v7: ref<test.main.Counter, managed, mutable, local> = variant.payload v6, 1
    v8: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } = variant.new 1, v7
    local.set l1, v8
    jump b3

b6:
    v9: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } = variant.new 0
    local.set l1, v9
    jump b3

b7:
    v14: int32 = 1
    return v14

b8:
    v15: variant<uint2> { 0uint2 = void; 1uint2 = ref<test.main.Counter, managed, mutable, local>; 2uint2 = null; } = local.get l0
    v16: ref<test.main.Counter, managed, mutable, local> = variant.payload v15, 1
    v17: ref<int32, borrowed, 'managed, readonly, local> = field.project v16, 0
    v18: int32 = load v17
    return v18
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@14 size=8 align=8
/// @layout.discriminant owner=type@14 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@14 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@14 index=1 discriminant=1 payload_offset=0
/// @layout.case owner=type@14 index=2 discriminant=2 payload_offset=0
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

    session.assert_mir_function("main.ds", "test.main.Chain.constructor", r#"
type test.main.Chain {
    next: variant<uint1> { 0uint1 = ref<test.main.Chain, managed, mutable, local>; 1uint1 = null; };
    weight: int32;
}

function test.main.Chain.constructor<'a>(v0: ref<uninit<test.main.Chain>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Chain>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Chain>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Chain>, borrowed, 'a, mutable, local> = local.get l1
    v3: null = zeroed
    v4: variant<uint1> { 0uint1 = ref<test.main.Chain, managed, mutable, local>; 1uint1 = null; } = variant.new 1
    v5: ref<uninit<variant<uint1> { 0uint1 = ref<test.main.Chain, managed, mutable, local>; 1uint1 = null; }>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v5, v4
    v6: ref<uninit<test.main.Chain>, borrowed, 'a, mutable, local> = local.get l1
    v7: int32 = local.get l0
    v8: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v6, 1
    store v8, v7
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
        "main.ds",
        "test.main.total",
        r#"
type test.main.Chain {
    next: variant<uint1> { 0uint1 = ref<test.main.Chain, managed, mutable, local>; 1uint1 = null; };
    weight: int32;
}

function test.main.total(v0: ref<test.main.Chain, managed, mutable, local>): int32 {
    local l0: ref<test.main.Chain, managed, mutable, local>

entry(v0: ref<test.main.Chain, managed, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Chain, managed, mutable, local> = local.get l0
    v2: ref<int32, borrowed, 'managed, readonly, local> = field.project v1, 1
    v3: int32 = load v2
    return v3
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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.peek",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.peek<'a>(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>): int32 {
    local l0: ref<test.main.Counter, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Counter, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<int32, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.main",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.main(v0: ref<test.main.Counter, managed, mutable, local>): int32 {
    local l0: ref<test.main.Counter, managed, mutable, local>

entry(v0: ref<test.main.Counter, managed, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Counter, managed, mutable, local> = local.get l0
    v2: ref<test.main.Counter, borrowed, 'managed, readonly, local> = cast.bit v1 -> ref<test.main.Counter, borrowed, 'managed, readonly, local>
    v3: int32 = call test.main.peek(v2): <'a>(ref<test.main.Counter, borrowed, 'a, readonly, local>) => int32
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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.own", r#"
type test.main.Counter {
    count: int32;
}

function test.main.own(v0: int32): int32 {
    local l0: int32
    local l1: test.main.Counter
    local l2: test.main.Counter

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: ref<test.main.Counter, borrowed, 'frame, mutable, frame> = local.address l1
    v3: ref<uninit<test.main.Counter>, borrowed, 'frame, mutable, frame> = cast.bit v2 -> ref<uninit<test.main.Counter>, borrowed, 'frame, mutable, frame>
    call test.main.Counter.constructor(v3, v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, int32) => void
    v4: ref<test.main.Counter, borrowed, 'frame, mutable, frame> = intrinsic.memory.raw.transmute(v3)
    v5: test.main.Counter = load v4
    local.set l2, v5
    v6: ref<test.main.Counter, borrowed, 'frame, readonly, frame> = local.project l2
    v7: ref<int32, borrowed, 'frame, readonly, frame> = field.project v6, 0
    v8: int32 = load v7
    return v8
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
        "main.ds",
        "test.main.Counter.constructor",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 3
    v3: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.make",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.make(): test.main.Counter {
    local l0: test.main.Counter

entry:
    v0: ref<test.main.Counter, borrowed, 'frame, mutable, frame> = local.address l0
    v1: ref<uninit<test.main.Counter>, borrowed, 'frame, mutable, frame> = cast.bit v0 -> ref<uninit<test.main.Counter>, borrowed, 'frame, mutable, frame>
    call test.main.Counter.constructor(v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>) => void
    v2: ref<test.main.Counter, borrowed, 'frame, mutable, frame> = intrinsic.memory.raw.transmute(v1)
    v3: test.main.Counter = load v2
    return v3
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

    session.assert_mir_function(
        "main.ds",
        "test.main.Base.constructor",
        r#"
type test.main.Base {
    tag: int32;
}

function test.main.Base.constructor<'a>(v0: ref<uninit<test.main.Base>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Base>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Base>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Base>, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 1
    v3: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
    return
}

/// @layout.struct name=test.main.Base size=4 align=4
/// @layout.field owner=test.main.Base index=0 name=tag offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Derived.constructor", r#"
type test.main.Base {
    tag: int32;
}

type test.main.Derived {
    tag: int32;
    extra: int32;
}

function test.main.Derived.constructor<'a>(v0: ref<uninit<test.main.Derived>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Derived>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Derived>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Derived>, borrowed, 'a, mutable, local> = local.get l0
    v2: ref<uninit<test.main.Base>, borrowed, 'a, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Base>, borrowed, 'a, mutable, local>
    call test.main.Base.constructor(v2): <'a>(ref<uninit<test.main.Base>, borrowed, 'a, mutable, local>) => void
    v3: ref<uninit<test.main.Derived>, borrowed, 'a, mutable, local> = local.get l0
    v4: int32 = 2
    v5: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v3, 1
    store v5, v4
    v6: void = zeroed
    return
}

/// @layout.struct name=test.main.Base size=4 align=4
/// @layout.field owner=test.main.Base index=0 name=tag offset=0 size=4 align=4
/// @layout.struct name=test.main.Derived size=8 align=4
/// @layout.field owner=test.main.Derived index=0 name=tag offset=0 size=4 align=4
/// @layout.field owner=test.main.Derived index=1 name=extra offset=4 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.build",
        r#"
type test.main.Derived {
    tag: int32;
    extra: int32;
}

function test.main.build(): test.main.Derived {
    local l0: test.main.Derived

entry:
    v0: ref<test.main.Derived, borrowed, 'frame, mutable, frame> = local.address l0
    v1: ref<uninit<test.main.Derived>, borrowed, 'frame, mutable, frame> = cast.bit v0 -> ref<uninit<test.main.Derived>, borrowed, 'frame, mutable, frame>
    call test.main.Derived.constructor(v1): <'a>(ref<uninit<test.main.Derived>, borrowed, 'a, mutable, local>) => void
    v2: ref<test.main.Derived, borrowed, 'frame, mutable, frame> = intrinsic.memory.raw.transmute(v1)
    v3: test.main.Derived = load v2
    return v3
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
        "main.ds",
        "test.main.Failure.display",
        r#"
@languageItem("string.String")
type String;

type test.main.Failure {
    message: ref<String, managed, mutable, local>;
}

function test.main.Failure.display<'a>(v0: ref<test.main.Failure, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.Failure, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Failure, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Failure, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: ref<String, managed, readonly, local> = load v2
    v4: ref<String, borrowed, 'a, readonly, local> = cast.bit v3 -> ref<String, borrowed, 'a, readonly, local>
    return v4
}

/// @layout.struct name=test.main.Failure size=8 align=8
/// @layout.field owner=test.main.Failure index=0 name=message offset=0 size=8 align=8
"#,
    );
    session.assert_mir_function("main.ds", "test.main.Failure.constructor", r#"
@languageItem("string.String")
type String;

type test.main.Failure {
    message: ref<String, managed, mutable, local>;
}

function test.main.Failure.constructor<'a>(v0: ref<uninit<test.main.Failure>, borrowed, 'a, mutable, local>, v1: ref<String, managed, mutable, local>): void {
    local l0: ref<String, managed, mutable, local>
    local l1: ref<uninit<test.main.Failure>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Failure>, borrowed, 'a, mutable, local>, v1: ref<String, managed, mutable, local>):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Failure>, borrowed, 'a, mutable, local> = local.get l1
    v3: ref<String, managed, mutable, local> = local.get l0
    v4: ref<uninit<ref<String, managed, mutable, local>>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.Failure size=8 align=8
/// @layout.field owner=test.main.Failure index=0 name=message offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.ds", "test.main.Failure.display", r#"
@languageItem("string.String")
type String;

type test.main.Failure {
    message: ref<String, managed, mutable, local>;
}

function test.main.Failure.display<'a>(v0: ref<test.main.Failure, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.Failure, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Failure, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Failure, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: ref<String, managed, readonly, local> = load v2
    v4: ref<String, borrowed, 'a, readonly, local> = cast.bit v3 -> ref<String, borrowed, 'a, readonly, local>
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

    session.assert_mir_function("main.ds", "test.main.Runtime.constructor", r#"
@languageItem("string.String")
type String;

type test.main.Runtime {
    id: int32;
    name: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; };
}

function test.main.Runtime.constructor<'a>(v0: ref<uninit<test.main.Runtime>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Runtime>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Runtime>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Runtime>, borrowed, 'a, mutable, local> = local.get l1
    v3: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 1
    v4: ref<uninit<variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }>, borrowed, 'a, mutable, local> = field.project v2, 1
    store v4, v3
    v5: ref<uninit<test.main.Runtime>, borrowed, 'a, mutable, local> = local.get l1
    v6: int32 = local.get l0
    v7: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v5, 0
    store v7, v6
    return
}

/// @layout.struct name=test.main.Runtime size=16 align=8
/// @layout.field owner=test.main.Runtime index=0 name=id offset=8 size=4 align=4
/// @layout.field owner=test.main.Runtime index=1 name=name offset=0 size=8 align=8
/// @layout.variant name=type@12 size=8 align=8
/// @layout.discriminant owner=type@12 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@12 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@12 index=1 discriminant=1 payload_offset=0
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
        "main.ds",
        "test.main.Counter.constructor",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 2
    v3: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
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
import { Slice } from "destack:collections";

class Holder {
    storage: ^[int32] = Slice.new();
}

function size(holder: &readonly Holder): isize {
    let view: &readonly [int32] = &readonly holder.storage;

    return view.size;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Holder.constructor", r#"
type test.main.Holder {
    storage: slice<int32, unique, mutable, local>;
}

function test.main.Holder.constructor<'a>(v0: ref<uninit<test.main.Holder>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Holder>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Holder>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Holder>, borrowed, 'a, mutable, local> = local.get l0
    v2: slice<int32, unique, mutable, local> = call Slice.new<int32>(): () => slice<int32, unique, mutable, local>
    v3: ref<uninit<slice<int32, unique, mutable, local>>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
    return
}

/// @layout.struct name=test.main.Holder size=16 align=8
/// @layout.field owner=test.main.Holder index=0 name=storage offset=0 size=16 align=8
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.size",
        r#"
type test.main.Holder {
    storage: slice<int32, unique, mutable, local>;
}

function test.main.size<'a>(v0: ref<test.main.Holder, borrowed, 'a, readonly, local>): isize {
    local l0: ref<test.main.Holder, borrowed, 'a, readonly, local>
    local l1: slice<int32, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Holder, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Holder, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<slice<int32, unique, readonly, local>, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: slice<int32, borrowed, 'a, readonly, local> = load v2
    local.set l1, v3
    v4: slice<int32, borrowed, 'a, readonly, local> = local.get l1
    v5: isize = call Slice.size.get<int32>(v4): <'a>(slice<int32, borrowed, 'a, readonly, local>) => isize
    return v5
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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: boolean): void {
    local l0: boolean
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: boolean):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = 1
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    v5: boolean = local.get l0
    branch v5 => b1 | b2

b1:
    v6: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v7: int32 = 0
    v8: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v6, 0
    store v8, v7
    jump b2

b2:
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.make", r#"
type test.main.Counter {
    count: int32;
}

function test.main.make(v0: boolean): ref<test.main.Counter, managed, mutable, local> {
    local l0: boolean

entry(v0: boolean):
    local.set l0, v0
    v1: boolean = local.get l0
    v2: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v2 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v3, v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, boolean) => void
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
        "main.ds",
        "test.main.Counter.constructor",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 1
    v3: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
    v4: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l0
    v5: ref<uninit<int32>, borrowed, 'a, readonly, local> = field.project v4, 0
    v6: int32 = load v5
    v7: int32 = 1
    v8: int32 = add v6, v7
    v9: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v4, 0
    store v9, v8
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.make",
        r#"
type test.main.Counter {
    count: int32;
}

function test.main.make(): ref<test.main.Counter, managed, mutable, local> {
entry:
    v0: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v0 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>) => void
    return v0
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#,
    );
}
