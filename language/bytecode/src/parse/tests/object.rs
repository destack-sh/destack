use destack_core::SectionStorage;

use crate::{FunctionId, Object, Opcode, Scalar};

use super::TestParser;

/// Load one parsed object directly from its immutable section image.
#[test]
fn test_load_object_image() {
    let image = TestParser::new(
        r#"
function f0 {    constant.int32 r0, 42
    return r0
}
"#,
    )
    .parse();
    let storage = SectionStorage::from_bytes(image.bytes());
    let object = Object::load(storage).expect("load object image");

    // preserve the complete encoded object and its navigable sections
    assert_eq!(object.bytes(), image.bytes());
    assert_eq!(object.functions()[0].register_count, 1);

    // execute normal object navigation over the mapped instruction stream
    let opcodes = object
        .instructions(FunctionId(0))
        .expect("defined function")
        .map(|instruction| instruction.expect("valid instruction").opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![Opcode::constant(Scalar::Int32), Opcode::RETURN]
    );
}
