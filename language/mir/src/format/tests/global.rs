use super::assert_format;

/// Format mutable globals and their addresses canonically.
#[test]
fn test_format_global_variable() {
    assert_format(
        r#"
global counter: int32 = zeroinit

function increment(): void {
entry:
    v0: ref<int32, borrowed, 'static, mutable> = address @counter
    v1: int32 = load (*v0)
    v2: int32 = 1
    v3: int32 = add v1, v2
    store (*v0), v3
    return
}
"#,
    );
}

/// Formats Program constants and loads canonically.
#[test]
fn test_format_global_constant() {
    assert_format(
        r#"
constant MAGIC: int64 = 42

function getMagic(): int64 {
entry:
    v0: ref<int64, borrowed, 'static, readonly> = address @MAGIC
    v1: int64 = load (*v0)
    return v1
}
"#,
    );
}

/// Formats byte constants and escapes canonically.
#[test]
fn test_format_string_constant() {
    assert_format(
        r#"
constant stringLiteralHelloWorldNl: [uint8; 11] = b"hello\nworld"

function escapeTest(): void {
entry:
    v0: ref<[uint8; 11], borrowed, 'static, readonly> = address @stringLiteralHelloWorldNl
    v1: [uint8; 11] = load (*v0)
    return
}
"#,
    );
}

/// Formats shared globals as declaration modifiers.
#[test]
fn test_format_shared_global() {
    assert_format(
        r#"
shared global counter: int32 = zeroinit
readonly shared global limit: int32 = 42
"#,
    );
}

/// Formats value-form literal constants canonically.
#[test]
fn test_format_literal_value_constants() {
    assert_format(
        r#"
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, managed, mutable, local>;
}

constant bigint.0: int64 = 100n
constant string.0: String = "vector \"quoted\""
"#,
    );
}
