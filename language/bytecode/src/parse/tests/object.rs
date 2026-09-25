use std::mem::{offset_of, size_of};

use tspp_core::{SectionImageError, SectionStorage};

use crate::{Function, FunctionId, Object, ObjectLoadError, Opcode, Scalar};

use super::TestParser;

/// Load one parsed object directly from its immutable section image.
#[test]
fn test_load_object_image() {
    let image = TestParser::new(
        r#"
function f0 {
    constant.int32 r0, 42
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

    // reject an invalid nested optional tag before constructing a Function reference
    let function_offset =
        object.functions().as_ptr().cast::<u8>() as usize - object.bytes().as_ptr() as usize;
    let mut invalid_tag = object.bytes().to_vec();
    invalid_tag[function_offset..function_offset + size_of::<u32>()]
        .copy_from_slice(&2_u32.to_ne_bytes());
    let error = Object::from_bytes(&invalid_tag).expect_err("reject invalid function tag");
    assert_eq!(
        error,
        ObjectLoadError::Image(SectionImageError::InvalidEntry)
    );

    // reject a sibling range that escapes the operation column
    let operations_offset = function_offset + offset_of!(Function, operations);
    let mut invalid_range = object.bytes().to_vec();
    invalid_range[operations_offset..operations_offset + size_of::<u32>()]
        .copy_from_slice(&u32::MAX.to_ne_bytes());
    let error = Object::from_bytes(&invalid_range).expect_err("reject invalid operation range");
    assert_eq!(error, ObjectLoadError::InvalidRange);
}
