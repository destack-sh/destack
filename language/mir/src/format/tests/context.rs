use super::assert_format;

/// Formats execution context operations canonically.
#[test]
fn test_format_context_family() {
    assert_format(
        r#"
type Context { }

type Variable { }

type ContextNode {
    parent: ref<Context, managed, readonly>;
    variable: ref<Variable, managed, readonly>;
    value: int32;
}

function scope(v0: ref<Variable, managed, readonly>, v1: int32): int32 {
entry(v0: ref<Variable, managed, readonly>, v1: int32):
    v2: ref<Context, managed, readonly> = context.current
    v3: ref<Context, managed, readonly> = context.bind v2, v0, v1, ContextNode
    v4: ref<Context, managed, readonly> = context.replace v3
    v5: int32 = context.get v3, v0, v1, ContextNode
    v6: ref<Context, managed, readonly> = context.replace v4
    return v5
}
"#,
    );
}
