use crate::tests::TestProgram;

/// Emit dynamic binding and direct descriptor projections.
#[test]
fn test_emit_bytecode_dynamic() {
    let mut program = TestProgram::mir(
        r#"
type Writer { }
type FileWriter { }

export function inspect(v0: ref<FileWriter, managed, readonly>): typeId {
entry(v0: ref<FileWriter, managed, readonly>):
    v1: dynamic<Writer, managed, readonly> = dynamic.bind v0, FileWriter
    v2: ref<void, managed, readonly> = dynamic.payload v1
    v3: typeId = dynamic.type v1
    return v3
}
"#,
    );
    program.mark_dynamic("FileWriter", "Writer");

    program.assert_bytecode(
        r#"function inspect {
    dynamic.bind r1:r2, r0, d0
    extract r0, r1:r2, 0:8
    dynamic.type r0, r1:r2
    return r0
}
"#,
    );
}
