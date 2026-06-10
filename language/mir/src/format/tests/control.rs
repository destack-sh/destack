use super::assert_format;

/// Formats switch based control flow canonically.
#[test]
fn test_format_switch() {
    assert_format(
        r#"
function dispatch(value0: int32): int32 {
entry0(value0: int32):
    switch value0, block3, 0 => block1, 1 => block2

block1:
    value1: int32 = 100int32
    return value1

block2:
    value2: int32 = 200int32
    return value2

block3:
    value3: int32 = 0int32
    return value3
}
"#,
    );
}

/// Formats yielding control flow canonically.
#[test]
fn test_format_yield() {
    assert_format(
        r#"
function yieldOnce(value0: int32): int32 {
entry0(value0: int32):
    value1: int32 = 5int32
    yield value1 -> block1(value0)

block1(value2: int32, value3: int32):
    value4: int32 = int.add value2, value3
    return value4
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

function suspends(value0: int32): int32 {
entry0(value0: int32):
    value1: int32 = 5int32
    yield value1 -> block1(value0) | block2

block1(value2: int32, value3: int32):
    call callee(value3): (int32) -> int32 -> block3 | block2

block2:
    unwind.resume

block3(value4: int32):
    return value4
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

function caller(value0: int32): int32 {
entry0(value0: int32):
    call callee(value0): (int32) -> int32 -> block1

block1(value1: int32):
    return value1
}
"#,
    );
}

/// Formats trap terminators canonically.
#[test]
fn test_format_trap() {
    assert_format(
        r#"
function trapper(value0: ref<void, managed, readonly>): void {
entry0(value0: ref<void, managed, readonly>):
    panic value0
}
"#,
    );
}

/// Formats checked control flow canonically.
#[test]
fn test_format_check_type_guards() {
    assert_format(
        r#"
function guard(value0: uint32, value1: ref<void, managed>): int32 {
entry0(value0: uint32, value1: ref<void, managed>):
    value2: boolean = int.eq value0, value0
    check dynamicType value0, int32 -> block1, block4

block1:
    value3: boolean = int.eq value0, value0
    check variantTag value0, 1uint32 -> block4, block5

block2:
    value4: boolean = int.eq value0, value0
    check receiverType value1, int32 -> block2, block4

block3:
    value5: boolean = int.eq value0, value0
    check interfaceConformance value1, int32 -> block3, block5

block4:
    value6: int32 = 0int32
    return value6

block5:
    unreachable
}
"#,
    );
}
