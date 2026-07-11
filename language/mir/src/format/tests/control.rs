use super::assert_format;

/// Formats switch based control flow canonically.
#[test]
fn test_format_switch() {
    assert_format(
        r#"
function dispatch(v0: int32): int32 {
entry(v0: int32):
    switch v0, b3, 0 => b1, 1 => b2

b1:
    v1: int32 = 100
    return v1

b2:
    v2: int32 = 200
    return v2

b3:
    v3: int32 = 0
    return v3
}
"#,
    );
}

/// Formats yielding control flow canonically.
#[test]
fn test_format_yield() {
    assert_format(
        r#"
function yieldOnce(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 5
    yield v1 => b1(v0)

b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}
"#,
    );
}

/// Formats suspension and call unwind alternatives canonically.
#[test]
fn test_format_unwind_continuation() {
    assert_format(
        r#"
external function callee(int32): int32

function suspends(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 5
    yield v1 => b1(v0) | b2

b1(v2: int32, v3: int32):
    invoke callee(v3) => b3 | b2

b2:
    unwind.resume

b3(v4: int32):
    return v4
}
"#,
    );
}

/// Formats invokes canonically.
#[test]
fn test_format_invoke() {
    assert_format(
        r#"
external function callee(int32): int32

function caller(v0: int32): int32 {
entry(v0: int32):
    invoke callee(v0) => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}
"#,
    );
}

/// Formats every open invoke and tail-call dispatch canonically.
#[test]
fn test_format_open_invoke_and_tail_call_families() {
    assert_format(
        r#"
external function callee(int32): int32

function invokeIndirect(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    invoke.indirect v0(v1): (int32) => int32 => b1 | b2

b1(v2: int32):
    return v2

b2:
    unwind.resume
}

function invokeVirtual(v0: int32): int32 {
entry(v0: int32):
    invoke.virtual v0, int32, 0(v0): (int32) => int32 => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}

function invokeDynamic(v0: int32): int32 {
entry(v0: int32):
    invoke.dynamic v0, int32, 0(v0): (int32) => int32 => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}

function tailDirect(v0: int32): int32 {
entry(v0: int32):
    tail.call callee(v0)
}

function tailIndirect(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    tail.call.indirect v0(v1): (int32) => int32
}

function tailVirtual(v0: int32): int32 {
entry(v0: int32):
    tail.call.virtual v0, int32, 0(v0): (int32) => int32
}

function tailDynamic(v0: int32): int32 {
entry(v0: int32):
    tail.call.dynamic v0, int32, 0(v0): (int32) => int32
}
"#,
    );
}

/// Formats trap terminators canonically.
#[test]
fn test_format_trap() {
    assert_format(
        r#"
function trapper(v0: ref<void, managed, readonly>): void {
entry(v0: ref<void, managed, readonly>):
    panic v0
}
"#,
    );
}

/// Formats checked control flow canonically.
#[test]
fn test_format_check_type_guards() {
    assert_format(
        r#"
function guard(v0: ref<void, managed, mutable>): int32 {
entry(v0: ref<void, managed, mutable>):
    check is.type v0, int32 => b1, b3

b1:
    check is.subtype v0, int32 => b2, b3

b2:
    v1: int32 = 0
    return v1

b3:
    unreachable
}
"#,
    );
}
