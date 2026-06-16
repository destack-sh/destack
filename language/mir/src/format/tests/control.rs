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
    call callee(v3) => b3 | b2

b2:
    unwind.resume

b3(v4: int32):
    return v4
}
"#,
    );
}

/// Formats call terminators canonically.
#[test]
fn test_format_call_terminator() {
    assert_format(
        r#"
external function callee(int32): int32

function caller(v0: int32): int32 {
entry(v0: int32):
    call callee(v0) => b1

b1(v1: int32):
    return v1
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
function guard(v0: uint32, v1: ref<void, managed>): int32 {
entry(v0: uint32, v1: ref<void, managed>):
    v2: boolean = int.eq v0, v0
    check dynamic.type v0, int32 => b1, b4

b1:
    v3: boolean = int.eq v0, v0
    check variant.tag v0, 1uint32 => b4, b5

b2:
    v4: boolean = int.eq v0, v0
    check receiver.type v1, int32 => b2, b4

b3:
    v5: boolean = int.eq v0, v0
    check interface.conformance v1, int32 => b3, b5

b4:
    v6: int32 = 0
    return v6

b5:
    unreachable
}
"#,
    );
}
