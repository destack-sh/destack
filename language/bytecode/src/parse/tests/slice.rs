use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse strided slice views and physical length extraction.
#[test]
fn test_parse_slice_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0 {
    slice.view r3:r4, r0:r1, 8, r2, r2
    extract r5, r3:r4, 8:8
    return r3:r5
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![Opcode::SLICE_VIEW, Opcode::EXTRACT, Opcode::RETURN]
    );
    assert_eq!(object.functions()[0].register_count, 6);
}
