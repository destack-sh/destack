use crate::tests::TestProgram;

/// Emit execution context operations with one concrete node allocation site.
#[test]
fn test_emit_bytecode_context() {
    let program = TestProgram::mir(
        r#"
type Context { }

type Variable { }

type ContextNode {
    parent: ref<Context, managed, readonly>;
    variable: ref<Variable, managed, readonly>;
    value: int32;
}

export function scope(v0: ref<Variable, managed, readonly>, v1: int32): int32 {
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

    program.assert_bytecode(
        r#"
function scope {
    context.current r2
    context.bind r3, r2, r0, r1, a0, 16
    context.replace r2, r3
    context.get r4, r3, r0, r1, 16
    context.replace r0, r2
    return r4
}
"#,
    );
}
