use super::assert_format;

/// Formats scalar static arguments and static function references canonically.
#[test]
fn test_format_static_scalars() {
    assert_format(
        r#"
function select<4>(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function values<null, undefined, true, -4, 8n, 1.5, -0.0, 'x', "fast", /a\\/b/gi>(): void {
entry:
    return
}

function use(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call select<4>(v0): (int32) => int32
    return v1
}
"#,
    );
}

/// Formats structural static arguments without MIR-only constructor syntax.
#[test]
fn test_format_static_aggregates() {
    assert_format(
        r#"
function configured<[1, 2], [0; 32], (1, true), { name: "fast", enabled: true }>(): void {
entry:
    return
}
"#,
    );
}

/// Distinguishes type values from nominal static values canonically.
#[test]
fn test_format_static_types() {
    assert_format(
        r#"
type UserId = newtype<int32>;

type Config {
    enabled: boolean;
}

function configured<int32, (int32, boolean), UserId(42), Config { enabled: true }>(): void {
entry:
    return
}
"#,
    );
}
