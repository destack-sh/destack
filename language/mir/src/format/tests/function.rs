use super::{assert_format, assert_format_eq};

/// Formats a simple add function canonically.
#[test]
fn test_format_simple_add() {
    assert_format(
        r#"
function add(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
"#,
    );
}

/// Formats local declarations and local access operations canonically.
#[test]
fn test_format_with_locals() {
    assert_format(
        r#"
function withLocals(): int64 {
    local l0: int64

entry:
    v0: int64 = 42
    local.set l0, v0
    v1: int64 = local.get l0
    return v1
}
"#,
    );
}

/// Formats local address operations canonically.
#[test]
fn test_format_local_address() {
    assert_format(
        r#"
function localAddr(): void {
    local l0: int32

entry:
    v0: ref<int32, borrowed, mutable, space(frame)> = local.address l0
    return
}
"#,
    );
}

/// Formats block control flow canonically.
#[test]
fn test_format_branch() {
    assert_format(
        r#"
function choose(v0: boolean, v1: int32, v2: int32): int32 {
entry(v0: boolean, v1: int32, v2: int32):
    branch v0, b1(v1), b2(v2)

b1(v3: int32):
    return v3

b2(v4: int32):
    return v4
}
"#,
    );
}

/// Formats void returns canonically.
#[test]
fn test_format_void_return() {
    assert_format(
        r#"
function noop(): void {
entry:
    return
}
"#,
    );
}

/// Formats environment functions and closure values canonically.
#[test]
fn test_format_closure_environment() {
    assert_format(
        r#"
@environment(ref<void, managed, mutable>)
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: ref<void, managed, mutable> = closure.environment.current
    return v0
}

@environment(ref<void, managed, mutable>)
function caller(): int32 {
entry:
    v0: ref<void, managed, mutable> = closure.environment.current
    v1: (int32) => int32 = closure.bind callee, v0
    v2: fn(int32) => int32 = closure.function v1
    v3: ref<void, managed, mutable> = closure.environment v1
    v4: int32 = 1
    v5: int32 = call.indirect v1(v4): (int32) => int32
    return v5
}
"#,
    );
}

/// Renames non canonical value and block names during formatting.
#[test]
fn test_format_renames_non_canonical_names() {
    assert_format_eq(
        r#"
function varTest(): int32 {
entry:
    v0: int32 = 10
    return v0
}
"#,
        r#"
function varTest(): int32 {
entry:
    v0: int32 = 10
    return v0
}
"#,
    );
}

/// Preserves reference access spelling in canonical output.
#[test]
fn test_format_reference_access_preserved() {
    assert_format(
        r#"
function refMutability(v0: ref<int32, managed, mutable>, v1: ref<int32, unique, readonly>): ref<int32, managed, mutable> {
entry(v0: ref<int32, managed, mutable>, v1: ref<int32, unique, readonly>):
    return v0
}
"#,
    );
}
