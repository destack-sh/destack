use super::{assert_format, assert_format_to};

/// Formats a simple add function canonically.
#[test]
fn test_format_roundtrip_simple_add() {
    assert_format(
        r#"
function add(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = int.add value0, value1
    return value2
}
"#,
    );
}

/// Formats local declarations and local access operations canonically.
#[test]
fn test_format_roundtrip_with_locals() {
    assert_format(
        r#"
function withLocals(): int64 {
    local local0: int64, owned

entry0:
    value0: int64 = 42int64
    local.set local0, value0
    value1: int64 = local.get local0
    return value1
}
"#,
    );
}

/// Formats local address operations canonically.
#[test]
fn test_format_roundtrip_local_address() {
    assert_format(
        r#"
function localAddr(): void {
    local local0: int32, owned

entry0:
    value0: ref<int32, borrowed, addressSpace(stack)> = local.address local0
    return
}
"#,
    );
}

/// Formats block control flow canonically.
#[test]
fn test_format_roundtrip_branch() {
    assert_format(
        r#"
function choose(value0: boolean, value1: int32, value2: int32): int32 {
entry0(value0: boolean, value1: int32, value2: int32):
    branch value0, block1(value1), block2(value2)

block1(value3: int32):
    return value3

block2(value4: int32):
    return value4
}
"#,
    );
}

/// Formats void returns canonically.
#[test]
fn test_format_roundtrip_void_return() {
    assert_format(
        r#"
function noop(): void {
entry0:
    return
}
"#,
    );
}

/// Formats environment functions and bound closures canonically.
#[test]
fn test_format_roundtrip_function_environment() {
    assert_format(
        r#"
@environment(ref<void, managed>)
function callee(value0: int32): int32 {
entry0(value0: int32):
    value1: ref<void, managed> = function.environment
    return value0
}

@environment(ref<void, managed>)
function caller(): int32 {
entry0:
    value0: ref<void, managed> = function.environment
    value1: closure(int32) -> int32 = function.bind callee, value0
    value2: int32 = 1int32
    value3: int32 = call.indirect value1(value2): (int32) -> int32
    return value3
}
"#,
    );
}

/// Renames non canonical value and block names during formatting.
#[test]
fn test_format_roundtrip_renames_non_canonical_names() {
    assert_format_to(
        r#"
function varTest(): int32 {
b0:
    v0: int32 = 10int32
    return v0
}
"#,
        r#"
function varTest(): int32 {
entry0:
    value0: int32 = 10int32
    return value0
}
"#,
    );
}

/// Preserves reference mutability spelling in canonical output.
#[test]
fn test_format_roundtrip_reference_mutability_preserved() {
    assert_format(
        r#"
function refMutability(value0: ref<int32, managed>, value1: ref<int32, owned, readonly>): ref<int32, managed> {
entry0(value0: ref<int32, managed>, value1: ref<int32, owned, readonly>):
    return value0
}
"#,
    );
}
