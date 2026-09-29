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

    session.assert_mir_function("main.tspp", "test.main.make", r#"
export function test.main.make(): function<(int32) => int32, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.make.closure#0, v0
    return v1
}
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.make.closure#0",
        r#"
export function test.main.make.closure#0(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
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

    session.assert_mir_function("main.tspp", "test.main.make", r#"
export function test.main.make(): function<() => int32, repeatable, managed, mutable, local> {
entry:
    v0: int32 = 1
    v1: ref<{ value: int32 }, managed, mutable, local> = new.zeroed { value: int32 }, local
    store (*v1).0, v0
    v2: { ref<{ value: int32 }, managed, mutable, local> } = aggregate (v1)
    v3: uninit<ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local>> = new.uninit { ref<{ value: int32 }, managed, mutable, local> }, local
    store (*v3), v2
    v4: ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local> = new.complete v3
    v5: function<() => int32, repeatable, managed, mutable, local> = function.bind test.main.make.closure#0, v4
    return v5
}

/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@5 size=8 align=8
/// @layout.field owner=type@5 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.tspp", "test.main.make.closure#0", r#"
@environment(ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local>)
export function test.main.make.closure#0(): int32 {
entry:
    v0: ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<{ value: int32 }, managed, mutable, local> = load (*v0).0
    v2: int32 = load (*v1).0
    v3: int32 = 1
    v4: int32 = add v2, v3
    return v4
}

/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@5 size=8 align=8
/// @layout.field owner=type@5 index=0 offset=0 size=8 align=8
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

    session.assert_mir_function("main.tspp", "test.main.counter", r#"
export function test.main.counter(): function<() => int32, repeatable, managed, mutable, local> {
    local l0: function<() => int32, repeatable, managed, mutable, local>

entry:
    v0: int32 = 0
    v1: ref<{ count: int32 }, managed, mutable, local> = new.zeroed { count: int32 }, local
    store (*v1).0, v0
    v2: { ref<{ count: int32 }, managed, mutable, local> } = aggregate (v1)
    v3: uninit<ref<{ ref<{ count: int32 }, managed, mutable, local> }, managed, mutable, local>> = new.uninit { ref<{ count: int32 }, managed, mutable, local> }, local
    store (*v3), v2
    v4: ref<{ ref<{ count: int32 }, managed, mutable, local> }, managed, mutable, local> = new.complete v3
    v5: function<() => int32, repeatable, managed, mutable, local> = function.bind test.main.counter.closure#0, v4
    store l0, v5
    v6: function<() => int32, repeatable, managed, mutable, local> = load l0
    v7: function<() => int32, repeatable, borrowed, 'managed, mutable> = cast.bit v6 -> function<() => int32, repeatable, borrowed, 'managed, mutable>
    v8: int32 = call.indirect v7(): () => int32
    v9: function<() => int32, repeatable, managed, mutable, local> = load l0
    return v9
}

/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@5 size=8 align=8
/// @layout.field owner=type@5 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.tspp", "test.main.counter.closure#0", r#"
@environment(ref<{ ref<{ count: int32 }, managed, mutable, local> }, managed, mutable, local>)
export function test.main.counter.closure#0(): int32 {
entry:
    v0: ref<{ ref<{ count: int32 }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<{ count: int32 }, managed, mutable, local> = load (*v0).0
    v2: int32 = load (*v1).0
    v3: int32 = 1
    v4: int32 = add v2, v3
    store (*v1).0, v4
    v5: int32 = load (*v1).0
    return v5
}

/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@5 size=8 align=8
/// @layout.field owner=type@5 index=0 offset=0 size=8 align=8
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
        "main.tspp", r#"
@nocopy
type test.main.Cell = class { value: int32 };

export constructor test.main.Cell.constructor(v0: ref<uninit<test.main.Cell>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Cell>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Cell>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Cell>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

export function test.main.chain(v0: ref<test.main.Cell, managed, mutable, local>): function<() => function<() => int32, repeatable, managed, mutable, local>, repeatable, managed, mutable, local> {
    local l0: ref<test.main.Cell, managed, mutable, local>

entry(v0: ref<test.main.Cell, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Cell, managed, mutable, local> = load l0
    v2: ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { outer: ref<test.main.Cell, managed, mutable, local> }, local
    store (*v2).0, v1
    v3: int32 = 1
    v4: ref<test.main.Cell, managed, mutable, local> = new.zeroed test.main.Cell, local
    v5: ref<uninit<test.main.Cell>, borrowed, 'managed, mutable> = cast.bit v4 -> ref<uninit<test.main.Cell>, borrowed, 'managed, mutable>
    call test.main.Cell.constructor(v5, v3): (ref<uninit<test.main.Cell>, borrowed, 'managed, mutable>, int32) => void
    v6: ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { next: ref<test.main.Cell, managed, mutable, local> }, local
    store (*v6).0, v4
    v7: ptr<void, readonly> = null
    v8: function<() => function<() => int32, repeatable, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.chain.closure#0, v7
    return v8
}

export function test.main.chain.closure#0(): function<() => int32, repeatable, managed, mutable, local> {
entry:
    v0: int32 = 2
    v1: ref<test.main.Cell, managed, mutable, local> = new.zeroed test.main.Cell, local
    v2: ref<uninit<test.main.Cell>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Cell>, borrowed, 'managed, mutable>
    call test.main.Cell.constructor(v2, v0): (ref<uninit<test.main.Cell>, borrowed, 'managed, mutable>, int32) => void
    v3: ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { inner: ref<test.main.Cell, managed, mutable, local> }, local
    store (*v3).0, v1
    v4: ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { next: ref<test.main.Cell, managed, mutable, local> }, local
    v5: ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = new.zeroed { outer: ref<test.main.Cell, managed, mutable, local> }, local
    v6: { ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> } = aggregate (v4, v3, v5)
    v7: uninit<ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local>> = new.uninit { ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, local
    store (*v7), v6
    v8: ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local> = new.complete v7
    v9: function<() => int32, repeatable, managed, mutable, local> = function.bind test.main.chain.closure#0.closure#0, v8
    return v9
}

@environment(ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local>)
export function test.main.chain.closure#0.closure#0(): int32 {
entry:
    v0: ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load (*v0).0
    v2: ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load (*v0).1
    v3: ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load (*v0).2
    v4: ref<test.main.Cell, managed, mutable, local> = load (*v1).0
    v5: int32 = load (*v4).0
    v6: ref<test.main.Cell, managed, mutable, local> = load (*v2).0
    v7: int32 = load (*v6).0
    v8: int32 = add v5, v7
    v9: ref<test.main.Cell, managed, mutable, local> = load (*v3).0
    v10: int32 = load (*v9).0
    v11: int32 = add v8, v10
    return v11
}

/// @layout.class name=test.main.Cell size=8 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=4 size=4 align=4
/// @layout.class name=type@3 size=8 align=4
/// @layout.field owner=type@3 index=0 name=value offset=4 size=4 align=4
/// @layout.struct name=type@12 size=8 align=8
/// @layout.field owner=type@12 index=0 name=outer offset=0 size=8 align=8
/// @layout.struct name=type@15 size=8 align=8
/// @layout.field owner=type@15 index=0 name=next offset=0 size=8 align=8
/// @layout.struct name=type@18 size=8 align=8
/// @layout.field owner=type@18 index=0 name=inner offset=0 size=8 align=8
/// @layout.struct name=type@20 size=24 align=8
/// @layout.field owner=type@20 index=0 offset=0 size=8 align=8
/// @layout.field owner=type@20 index=1 offset=8 size=8 align=8
/// @layout.field owner=type@20 index=2 offset=16 size=8 align=8

/// @dispatch.virtual concrete=type@0 value=type@1
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.chain.closure#0.closure#0",
        r#"
@nocopy
type test.main.Cell = class { value: int32 };

@environment(ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local>)
export function test.main.chain.closure#0.closure#0(): int32 {
entry:
    v0: ref<{ ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local>, ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<{ next: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load (*v0).0
    v2: ref<{ inner: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load (*v0).1
    v3: ref<{ outer: ref<test.main.Cell, managed, mutable, local> }, managed, mutable, local> = load (*v0).2
    v4: ref<test.main.Cell, managed, mutable, local> = load (*v1).0
    v5: int32 = load (*v4).0
    v6: ref<test.main.Cell, managed, mutable, local> = load (*v2).0
    v7: int32 = load (*v6).0
    v8: int32 = add v5, v7
    v9: ref<test.main.Cell, managed, mutable, local> = load (*v3).0
    v10: int32 = load (*v9).0
    v11: int32 = add v8, v10
    return v11
}

/// @layout.class name=test.main.Cell size=8 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=4 size=4 align=4
/// @layout.struct name=type@12 size=8 align=8
/// @layout.field owner=type@12 index=0 name=outer offset=0 size=8 align=8
/// @layout.struct name=type@15 size=8 align=8
/// @layout.field owner=type@15 index=0 name=next offset=0 size=8 align=8
/// @layout.struct name=type@18 size=8 align=8
/// @layout.field owner=type@18 index=0 name=inner offset=0 size=8 align=8
/// @layout.struct name=type@20 size=24 align=8
/// @layout.field owner=type@20 index=0 offset=0 size=8 align=8
/// @layout.field owner=type@20 index=1 offset=8 size=8 align=8
/// @layout.field owner=type@20 index=2 offset=16 size=8 align=8
"#,
    );
}

/// Declare a lambda literal at the parameter list its context calls it with.
#[test]
fn test_declare_a_lambda_at_the_arity_its_context_calls() {
    let session = TestSession::single(
        r#"
function fold(step: (total: int32, value: int32, index: isize) => int32): int32 {
    step(0, 1, 2)
}

function sum(): int32 {
    fold((total, value) => total + value)
}
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
export function test.main.fold(v0: function<(int32, int32, isize) => int32, repeatable, managed, mutable, local>): int32 {
    local l0: function<(int32, int32, isize) => int32, repeatable, managed, mutable, local>

entry(v0: function<(int32, int32, isize) => int32, repeatable, managed, mutable, local>):
    store l0, v0
    v1: function<(int32, int32, isize) => int32, repeatable, managed, mutable, local> = load l0
    v2: int32 = 0
    v3: int32 = 1
    v4: isize = 2
    v5: function<(int32, int32, isize) => int32, repeatable, borrowed, 'managed, mutable> = cast.bit v1 -> function<(int32, int32, isize) => int32, repeatable, borrowed, 'managed, mutable>
    v6: int32 = call.indirect v5(v2, v3, v4): (int32, int32, isize) => int32
    return v6
}

export function test.main.sum(): int32 {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(int32, int32, isize) => int32, repeatable, managed, mutable, local> = function.bind test.main.sum.closure#0, v0
    v2: int32 = call test.main.fold(v1): (function<(int32, int32, isize) => int32, repeatable, managed, mutable, local>) => int32
    return v2
}

export function test.main.sum.closure#0(v0: int32, v1: int32, v2: isize): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32, v2: isize):
    store l0, v0
    store l1, v1
    v3: int32 = load l0
    v4: int32 = load l1
    v5: int32 = add v3, v4
    return v5
}
"#,
    );
}

/// Lend borrowed captures through an environment held in the creating frame.
#[test]
fn test_lend_borrowed_captures_through_a_frame_environment() {
    let session = TestSession::single(
        r#"
function run(): int32 {
    let count: int32 = 1;
    let step: int32 = 10;
    @capture({ default: "borrow", step: "copy" })
    const bump = (): void => {
        count = count + step;
    };
    bump();
    bump();
    return count;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.run",
        r#"
export function test.main.run(): int32 {
    local l0: int32
    local l1: int32
    local l2: { ref<int32, borrowed, 'frame, exclusive>, int32 }
    local l3: function<() => void, repeatable, borrowed, 'frame, exclusive>

entry:
    v0: int32 = 1
    store l0, v0
    v1: int32 = 10
    store l1, v1
    v2: ref<int32, borrowed, 'frame, exclusive> = address l0
    v3: int32 = load l1
    v4: { ref<int32, borrowed, 'frame, exclusive>, int32 } = aggregate (v2, v3)
    store l2, v4
    v5: ref<{ ref<int32, borrowed, 'frame, exclusive>, int32 }, borrowed, 'frame, exclusive> = address l2
    v6: function<() => void, repeatable, borrowed, 'frame, exclusive> = function.bind test.main.run.closure#0, v5
    store l3, v6
    v7: function<() => void, repeatable, borrowed, 'frame, exclusive> = address (*l3)
    v8: function<() => void, repeatable, borrowed, 'frame, exclusive> = address (*v7)
    call.indirect v8(): () => void
    v9: function<() => void, repeatable, borrowed, 'frame, exclusive> = address (*l3)
    v10: function<() => void, repeatable, borrowed, 'frame, exclusive> = address (*v9)
    call.indirect v10(): () => void
    v11: int32 = load l0
    return v11
}

/// @layout.struct name=type@2 size=16 align=8
/// @layout.field owner=type@2 index=0 offset=0 size=8 align=8
/// @layout.field owner=type@2 index=1 offset=8 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.run.closure#0",
        r#"
@environment(ref<{ ref<int32, borrowed, 'frame, exclusive>, int32 }, borrowed, 'frame, exclusive>)
export function test.main.run.closure#0(): void {
entry:
    v0: ref<{ ref<int32, borrowed, 'frame, exclusive>, int32 }, borrowed, 'frame, exclusive> = function.environment.current
    v1: ref<int32, borrowed, 'frame, exclusive> = address (*(*v0).0)
    v2: int32 = load (*v1)
    v3: int32 = load (*v0).1
    v4: int32 = add v2, v3
    store (*v1), v4
    return
}

/// @layout.struct name=type@2 size=16 align=8
/// @layout.field owner=type@2 index=0 offset=0 size=8 align=8
/// @layout.field owner=type@2 index=1 offset=8 size=4 align=4
"#,
    );
}
