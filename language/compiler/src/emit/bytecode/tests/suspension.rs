use crate::tests::TestProgram;

/// Emit await and yield suspension into explicit continuation edges.
#[test]
fn test_emit_suspension() {
    let program = TestProgram::mir(
        r#"
external function park(int32, uint64): void

async function wait(v0: int32): int32 {
entry(v0: int32):
    await park(v0) => resumed | failed

resumed(v1: int32):
    return v1

failed:
    unwind.resume
}

function* generate(v0: int32): int32 {
entry(v0: int32):
    yield v0 => resumed | failed

resumed(v1: int32):
    return v1

failed:
    unwind.resume
}
"#,
    );

    program.assert_bytecode(
        r#"
function wait {
    await r0, park, r0 => b0 | b1

b0:
    return r0

b1:
    unwind.resume
}

function generate {
    yield r0, r0 => b0 | b1

b0:
    return r0

b1:
    unwind.resume
}
"#,
    );
}
