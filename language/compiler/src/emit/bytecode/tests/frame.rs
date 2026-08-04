use destack_bytecode::{RegisterId, RegisterSpan};

use crate::tests::TestProgram;

/// Map one runtime poll onto the exact live bytecode registers at its resume point.
#[test]
fn test_emit_bytecode_frame_map() {
    let program = TestProgram::mir(
        r#"
export function advance(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    poll
    return v1
}
"#,
    );
    let object = program.assert_bytecode(
        r#"
function advance {
    int.add r1, r0, r0: int32
    poll
    return r1
}
"#,
    );
    let bytecode = object
        .bytecode()
        .expect("bytecode emission should attach bytecode");
    let maps = bytecode.frames();
    let registers = bytecode.registers();

    // retain only the result live when execution resumes after the poll
    assert_eq!(maps.len(), 1);
    assert_eq!(
        maps[0].registers(registers),
        &[RegisterSpan::new(RegisterId(1), 1)]
    );
}
