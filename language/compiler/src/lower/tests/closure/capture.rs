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

    session.assert_mir_lowered("main.ds", r#"
function test.main.make(): function<(int32) => int32, repeatable, managed, mutable> {
entry:
    v0: ref<void, managed, mutable, nullable> = null
    v1: function<(int32) => int32, repeatable, managed, mutable> = function.bind test.main.make.closure#0, v0
    return v1
}

function test.main.make.closure#0(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 2
    v2: int32 = int.mul v0, v1
    return v2
}
"#);
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

    session.assert_mir_lowered("main.ds", r#"
function test.main.make(): function<() => int32, repeatable, managed, mutable> {
entry:
    v0: int32 = 1
    v1: ref<{ value: int32 }, managed, mutable> = new.zeroed { value: int32 }
    v2: ref<int32, borrowed, mutable> = field.address v1, 0
    store v2, v0
    v3: function<() => int32, repeatable, managed, mutable> = function.bind test.main.make.closure#0, v1
    return v3
}

@environment(ref<{ value: int32 }, managed, mutable>)
function test.main.make.closure#0(): int32 {
entry:
    v0: ref<{ value: int32 }, managed, mutable> = function.environment.current
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = load v1
    v3: int32 = 1
    v4: int32 = int.add v2, v3
    return v4
}
/// @layout.struct name=type@9 size=4 align=4
/// @layout.field owner=type@9 index=0 name=value offset=0 size=4 align=4
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

    session.assert_mir_lowered("main.ds", r#"
function test.main.counter(): function<() => int32, repeatable, managed, mutable> {
    local l0: function<() => int32, repeatable, managed, mutable>

entry:
    v0: int32 = 0
    v1: ref<{ count: int32 }, managed, mutable> = new.zeroed { count: int32 }
    v2: ref<int32, borrowed, mutable> = field.address v1, 0
    store v2, v0
    v3: function<() => int32, repeatable, managed, mutable> = function.bind test.main.counter.closure#0, v1
    local.set l0, v3
    v4: function<() => int32, repeatable, managed, mutable> = local.get l0
    v5: int32 = call.indirect v4(): () => int32
    v6: function<() => int32, repeatable, managed, mutable> = local.get l0
    return v6
}

@environment(ref<{ count: int32 }, managed, mutable>)
function test.main.counter.closure#0(): int32 {
entry:
    v0: ref<{ count: int32 }, managed, mutable> = function.environment.current
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
/// @layout.struct name=type@9 size=4 align=4
/// @layout.field owner=type@9 index=0 name=count offset=0 size=4 align=4
"#);
}
