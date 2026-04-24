use super::assert_format;

/// Formats mutable globals and global address operations canonically.
#[test]
fn test_format_global_variable() {
    assert_format(
        r#"
global counter: int32 = zeroInit

function increment(): void {
entry0:
    value0: ref<int32, raw, space(global)> = global.address counter
    value1: int32 = load value0
    value2: int32 = 1int32
    value3: int32 = int.add value1, value2
    store value0, value3
    return
}
"#,
    );
}

/// Formats immutable globals and global constant reads canonically.
#[test]
fn test_format_global_constant() {
    assert_format(
        r#"
global MAGIC: int64, readonly = 42int64

function getMagic(): int64 {
entry0:
    value0: int64 = global.const MAGIC
    return value0
}
"#,
    );
}

/// Formats string constants and escapes canonically.
#[test]
fn test_format_string_constant() {
    assert_format(
        r#"
global stringLiteralHelloWorldNl: uint8[11], readonly = "hello\nworld"

function escapeTest(): void {
entry0:
    value0: uint8[11] = global.const stringLiteralHelloWorldNl
    return
}
"#,
    );
}
