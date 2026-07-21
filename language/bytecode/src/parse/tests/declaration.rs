use crate::{
    ConstantId, FunctionId, FunctionTypeId, GlobalId, GlobalLocation, Linkage, Scalar, TypeId,
    ValueType,
};

use super::TestParser;

/// Parse object declarations into their persisted tables.
#[test]
fn test_parse_object_declarations() {
    let object = TestParser::new(
        r#"
type User
type Consumer = (int32) => void

constant defaultUser, align(8) = bytes(1, 2, 3, 4)
constant alignedUser, align(8) = bytes(5, 6)
readonly global user: User = constant defaultUser
local global cachedUser: User = zero
external function consume(int32): void
"#,
    )
    .parse();
    // type and callable declarations
    assert_eq!(object.types().len(), 1);
    assert_eq!(object.string(object.types()[0].name), Some("User"));
    assert_eq!(object.function_types().len(), 1);
    assert_eq!(
        object
            .function_type(FunctionTypeId(0))
            .expect("function type")
            .parameters(object.value_types()),
        &[ValueType::scalar(Scalar::Int32)]
    );

    // constant and global definitions
    assert_eq!(object.constants().len(), 2);
    assert_eq!(object.constants()[0].alignment_bytes, 8);
    assert_eq!(
        object.constants()[0].bytes(object.constant_bytes()),
        &[1, 2, 3, 4]
    );
    assert_eq!(object.constants()[1].bytes.start, 8);
    assert_eq!(
        object.constants()[1].bytes(object.constant_bytes()),
        &[5, 6]
    );
    let global = object.global(GlobalId(0)).expect("global");
    assert_eq!(global.location, GlobalLocation::CONSTANT);
    assert_eq!(global.ty, TypeId(0));
    assert_eq!(global.initializer(), Some(ConstantId(0)));
    let cached = object.global(GlobalId(1)).expect("cached global");
    assert_eq!(cached.location, GlobalLocation::LOCAL_STATIC);
    assert_eq!(cached.initializer(), None);

    // external function declaration
    let function = object.function(FunctionId(0)).expect("function");
    assert_eq!(object.string(function.name), Some("consume"));
    assert_eq!(function.linkage, Linkage::EXTERNAL);
    assert!(function.code().is_none());
}
