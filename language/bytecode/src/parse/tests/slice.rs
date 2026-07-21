use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse slice views and lengths as two-word values.
#[test]
fn test_parse_slice_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
type Point

export function sliceRange(
    r0: slice<Point, managed, space(local)>,
    r2: uint64,
): (slice<Point, managed, space(local)>, uint64) {
    r3: slice<Point, managed, space(local)> = slice.view r0, r2, r2
    r5: uint64 = slice.length r3
    return r3, r5
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![Opcode::SLICE_VIEW, Opcode::SLICE_LENGTH, Opcode::RETURN]
    );
    assert_eq!(object.functions()[0].register_count, 6);
}
