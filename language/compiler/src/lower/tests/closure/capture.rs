use crate::tests::TestSession;

/// Bind a closure that captures nothing against a null environment.
#[test]
fn test_lower_a_closure_without_captures() {
    let session = TestSession::single(
        r#"
function make(): (value: int32) => int32 {
    return (value: int32): int32 => value * 2;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.make", r#"
function test.main.make(): function<(int32) => int32, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.make.closure#0, v0
    return v1
}

/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.make.closure#0",
        r#"
function test.main.make.closure#0(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: int32 = 2
    v3: int32 = mul v1, v2
    return v3
}
"#,
    );
}

/// Read a captured binding from the frame the enclosing scope allocated for it.
#[test]
fn test_share_a_captured_binding_between_closure_and_scope() {
    let session = TestSession::single(
        r#"
function make(): () => int32 {
    let value: int32 = 1;
    return (): int32 => value + 1;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.make", r#"
function test.main.make(): function<() => int32, repeatable, managed, mutable, local> {
entry:
    v0: int32 = 1
    v1: ref<{ value: int32 }, managed, mutable, local> = new.zeroed { value: int32 }
    v2: ref<int32, borrowed, 'managed, mutable, local> = field.project v1, 0
    store v2, v0
    v3: { ref<{ value: int32 }, managed, mutable, local> } = aggregate (v1)
    v4: ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local> = new.complete v3
    v5: function<() => int32, repeatable, managed, mutable, local> = function.bind test.main.make.closure#0, v4
    return v5
}

/// @layout.struct name=type@8 size=4 align=4
/// @layout.field owner=type@8 index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@15 size=8 align=8
/// @layout.field owner=type@15 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.ds", "test.main.make.closure#0", r#"
@environment(ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local>)
function test.main.make.closure#0(): int32 {
entry:
    v0: ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<ref<{ value: int32 }, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v0, 0
    v2: ref<{ value: int32 }, managed, mutable, local> = load v1
    v3: ref<int32, borrowed, 'managed, readonly, local> = field.project v2, 0
    v4: int32 = load v3
    v5: int32 = 1
    v6: int32 = add v4, v5
    return v6
}

/// @layout.struct name=type@8 size=4 align=4
/// @layout.field owner=type@8 index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@15 size=8 align=8
/// @layout.field owner=type@15 index=0 offset=0 size=8 align=8
"#);
}

/// Write a captured binding from inside the closure so the enclosing scope sees the update.
#[test]
fn test_write_a_captured_binding_from_the_closure() {
    let session = TestSession::single(
        r#"
function counter(): () => int32 {
    let count: int32 = 0;
    let step = (): int32 => {
        count = count + 1;
        return count;
    };
    step();
    return step;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.counter", r#"
function test.main.counter(): function<() => int32, repeatable, managed, mutable, local> {
    local l0: function<() => int32, repeatable, managed, mutable, local>

entry:
    v0: int32 = 0
    v1: ref<{ count: int32 }, managed, mutable, local> = new.zeroed { count: int32 }
    v2: ref<int32, borrowed, 'managed, mutable, local> = field.project v1, 0
    store v2, v0
    v3: { ref<{ count: int32 }, managed, mutable, local> } = aggregate (v1)
    v4: ref<{ ref<{ count: int32 }, managed, mutable, local> }, managed, mutable, local> = new.complete v3
    v5: function<() => int32, repeatable, managed, mutable, local> = function.bind test.main.counter.closure#0, v4
    local.set l0, v5
    v6: function<() => int32, repeatable, managed, mutable, local> = local.get l0
    v7: function<() => int32, repeatable, borrowed, 'managed, mutable, local> = cast.bit v6 -> function<() => int32, repeatable, borrowed, 'managed, mutable, local>
    v8: int32 = call.indirect v7(): () => int32
    v9: function<() => int32, repeatable, managed, mutable, local> = local.get l0
    return v9
}

/// @layout.struct name=type@8 size=4 align=4
/// @layout.field owner=type@8 index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@15 size=8 align=8
/// @layout.field owner=type@15 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.ds", "test.main.counter.closure#0", r#"
@environment(ref<{ ref<{ count: int32 }, managed, mutable, local> }, managed, mutable, local>)
function test.main.counter.closure#0(): int32 {
entry:
    v0: ref<{ ref<{ count: int32 }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<ref<{ count: int32 }, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v0, 0
    v2: ref<{ count: int32 }, managed, mutable, local> = load v1
    v3: ref<int32, borrowed, 'managed, readonly, local> = field.project v2, 0
    v4: int32 = load v3
    v5: int32 = 1
    v6: int32 = add v4, v5
    v7: ref<int32, borrowed, 'managed, mutable, local> = field.project v2, 0
    store v7, v6
    v8: ref<int32, borrowed, 'managed, readonly, local> = field.project v2, 0
    v9: int32 = load v8
    return v9
}

/// @layout.struct name=type@8 size=4 align=4
/// @layout.field owner=type@8 index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@15 size=8 align=8
/// @layout.field owner=type@15 index=0 offset=0 size=8 align=8
"#);
}

/// Bind the managed captures of a nested closure from each frame that holds one.
#[test]
fn test_lower_managed_captures_from_two_frames() {
    let session = TestSession::single(
        r#"
class Cell {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

function chain(outer: Cell): () => () => int32 {
    const next = new Cell(1);
    return () => {
        const inner = new Cell(2);
        return () => next.value + inner.value + outer.value;
    };
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds", r#"
type test.main.Cell {
    value: int32;
}

function test.main.Cell.constructor<'a>(v0: ref<uninit<test.main.Cell>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Cell>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Cell>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Cell>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

function test.main.chain(v0: ref<test.main.Cell, managed, mutable, local>): function<() => function<() => int32, repeatable, managed, mutable, local>, repeatable, managed, mutable, local> {
    local l0: ref<test.main.Cell, managed, mutable, local>

entry(v0: ref<test.main.Cell, managed, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Cell, managed, mutable, local> = local.get l0
    v2: ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { outer: ref<test.main.Cell, managed, mutable, local> }
    v3: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, mutable, local> = field.project v2, 0
    store v3, v1
    v4: int32 = 1
    v5: ref<test.main.Cell, managed, mutable, local> = new.zeroed test.main.Cell
    v6: ref<uninit<test.main.Cell>, borrowed, 'managed, mutable, local> = cast.bit v5 -> ref<uninit<test.main.Cell>, borrowed, 'managed, mutable, local>
    call test.main.Cell.constructor(v6, v4): <'a>(ref<uninit<test.main.Cell>, borrowed, 'a, mutable, local>, int32) => void
    v7: ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { next: ref<test.main.Cell, managed, mutable, local> }
    v8: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, mutable, local> = field.project v7, 0
    store v8, v5
    v9: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v10: function<() => function<() => int32, repeatable, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.chain.closure#0, v9
    return v10
}

function test.main.chain.closure#0(): function<() => int32, repeatable, managed, mutable, local> {
entry:
    v0: int32 = 2
    v1: ref<test.main.Cell, managed, mutable, local> = new.zeroed test.main.Cell
    v2: ref<uninit<test.main.Cell>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Cell>, borrowed, 'managed, mutable, local>
    call test.main.Cell.constructor(v2, v0): <'a>(ref<uninit<test.main.Cell>, borrowed, 'a, mutable, local>, int32) => void
    v3: ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { inner: ref<test.main.Cell, managed, mutable, local> }
    v4: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, mutable, local> = field.project v3, 0
    store v4, v1
    v5: ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { next: ref<test.main.Cell, managed, mutable, local> }
    v6: ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { outer: ref<test.main.Cell, managed, mutable, local> }
    v7: { ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> } = aggregate (v5, v3, v6)
    v8: ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local> = new.complete v7
    v9: function<() => int32, repeatable, managed, mutable, local> = function.bind test.main.chain.closure#0.closure#0, v8
    return v9
}

@environment(ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local>)
function test.main.chain.closure#0.closure#0(): int32 {
entry:
    v0: ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v0, 0
    v2: ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load v1
    v3: ref<ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v0, 1
    v4: ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load v3
    v5: ref<ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v0, 2
    v6: ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load v5
    v7: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v2, 0
    v8: ref<test.main.Cell, managed, mutable, local> = load v7
    v9: ref<int32, borrowed, 'managed, readonly, local> = field.project v8, 0
    v10: int32 = load v9
    v11: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v4, 0
    v12: ref<test.main.Cell, managed, mutable, local> = load v11
    v13: ref<int32, borrowed, 'managed, readonly, local> = field.project v12, 0
    v14: int32 = load v13
    v15: int32 = add v10, v14
    v16: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v6, 0
    v17: ref<test.main.Cell, managed, mutable, local> = load v16
    v18: ref<int32, borrowed, 'managed, readonly, local> = field.project v17, 0
    v19: int32 = load v18
    v20: int32 = add v15, v19
    return v20
}

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@32 size=8 align=8
/// @layout.field owner=type@32 index=0 name=outer offset=0 size=8 align=8
/// @layout.struct name=type@45 size=8 align=8
/// @layout.field owner=type@45 index=0 name=next offset=0 size=8 align=8
/// @layout.variant name=type@53 size=8 align=8
/// @layout.discriminant owner=type@53 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@53 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@53 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@63 size=8 align=8
/// @layout.field owner=type@63 index=0 name=inner offset=0 size=8 align=8
/// @layout.struct name=type@73 size=24 align=8
/// @layout.field owner=type@73 index=0 offset=0 size=8 align=8
/// @layout.field owner=type@73 index=1 offset=8 size=8 align=8
/// @layout.field owner=type@73 index=2 offset=16 size=8 align=8
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.chain.closure#0.closure#0",
        r#"
type test.main.Cell {
    value: int32;
}

@environment(ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local>)
function test.main.chain.closure#0.closure#0(): int32 {
entry:
    v0: ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v0, 0
    v2: ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load v1
    v3: ref<ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v0, 1
    v4: ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load v3
    v5: ref<ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v0, 2
    v6: ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load v5
    v7: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v2, 0
    v8: ref<test.main.Cell, managed, mutable, local> = load v7
    v9: ref<int32, borrowed, 'managed, readonly, local> = field.project v8, 0
    v10: int32 = load v9
    v11: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v4, 0
    v12: ref<test.main.Cell, managed, mutable, local> = load v11
    v13: ref<int32, borrowed, 'managed, readonly, local> = field.project v12, 0
    v14: int32 = load v13
    v15: int32 = add v10, v14
    v16: ref<ref<test.main.Cell, managed, mutable, local>, borrowed, 'managed, readonly, local> = field.project v6, 0
    v17: ref<test.main.Cell, managed, mutable, local> = load v16
    v18: ref<int32, borrowed, 'managed, readonly, local> = field.project v17, 0
    v19: int32 = load v18
    v20: int32 = add v15, v19
    return v20
}

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@32 size=8 align=8
/// @layout.field owner=type@32 index=0 name=outer offset=0 size=8 align=8
/// @layout.struct name=type@45 size=8 align=8
/// @layout.field owner=type@45 index=0 name=next offset=0 size=8 align=8
/// @layout.struct name=type@63 size=8 align=8
/// @layout.field owner=type@63 index=0 name=inner offset=0 size=8 align=8
/// @layout.struct name=type@73 size=24 align=8
/// @layout.field owner=type@73 index=0 offset=0 size=8 align=8
/// @layout.field owner=type@73 index=1 offset=8 size=8 align=8
/// @layout.field owner=type@73 index=2 offset=16 size=8 align=8
"#,
    );
}
