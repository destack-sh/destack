use crate::tests::TestProgram;

/// Emit one direct call to the generated frame destructor.
#[test]
fn test_emit_bytecode_drop() {
    let mut program = TestProgram::mir(
        r#"
type Resource {
    int32;
}

function destroy(v0: Resource): void {
entry(v0: Resource):
    return
}

export function release(v0: Resource): void {
entry(v0: Resource):
    drop v0
    return
}
"#,
    );
    program.mark_destructor("Resource", "destroy");

    program.assert_bytecode(
        r#"function destroy {
    return
}

function release {
    drop r0, destroy
    return
}
"#,
    );
}
