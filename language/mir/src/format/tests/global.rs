use super::assert_format;

/// Formats mutable globals and global address operations canonically.
#[test]
fn test_format_global_variable() {
    assert_format(
        r#"
global counter: int32 = zeroInit

function increment(): void {
entry0:
    value0: ref<int32, raw> = global.address counter
    value1: int32 = load value0
    value2: int32 = 1int32
    value3: int32 = int.add value1, value2
    store value0, value3
    return
}
"#,
    );
}

/// Formats immutable globals and loads canonically.
#[test]
fn test_format_global_constant() {
    assert_format(
        r#"
readonly global MAGIC: int64 = 42int64

function getMagic(): int64 {
entry0:
    value0: ref<int64, raw, readonly> = global.address MAGIC
    value1: int64 = load value0
    return value1
}
"#,
    );
}

/// Formats string constants and escapes canonically.
#[test]
fn test_format_string_constant() {
    assert_format(
        r#"
readonly global stringLiteralHelloWorldNl: [uint8; 11] = b"hello\nworld"

function escapeTest(): void {
entry0:
    value0: ref<[uint8; 11], raw, readonly> = global.address stringLiteralHelloWorldNl
    value1: [uint8; 11] = load value0
    return
}
"#,
    );
}
