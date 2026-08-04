use crate::tests::TestProgram;

/// Emit exact counter and sample instrumentation sites.
#[test]
fn test_emit_bytecode_profile() {
    let program = TestProgram::mir(
        r#"
export function observe(v0: int32): int32 {
entry(v0: int32):
    profile.increment counter(0)
    profile.sample sampler(0), v0
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function observe {
    profile.increment c0
    profile.sample s0, r0
    return r0
}
"#,
    );
}
