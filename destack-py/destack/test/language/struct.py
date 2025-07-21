from destack.language import (
    BuiltinDefinition,
    Session,
    Struct,
    StructType,
)


def test_struct_inheritance(session: Session):
    """Test the Struct inheritance hierarchy."""
    assert Struct.metatype == StructType.STRUCT
    assert Struct.__is_abstract__
    assert BuiltinDefinition.metatype == StructType.BUILTIN_DEFINITION
    assert BuiltinDefinition.__is_abstract__
    assert BuiltinDefinition.__base_type__ == StructType.STRUCT
    assert BuiltinDefinition.__inherits__ == (StructType.STRUCT,)
    assert BuiltinDefinition.__extended_by__ == (
        StructType.NODE_DEFINITION,
        StructType.TRAIT_DEFINITION,
        StructType.STRUCT_DEFINITION,
        StructType.ENUM_DEFINITION,
        StructType.PROPERTY_DEFINITION,
        StructType.OPTION_DEFINITION,
        StructType.TAG_DEFINITION,
        StructType.INDEX_DEFINITION,
        StructType.CONSTRAINT_DEFINITION,
        StructType.PERMISSION_DEFINITION,
        StructType.METHOD_DEFINITION,
        StructType.MIGRATION_DEFINITION,
        StructType.MIGRATION_OPERATION_DEFINITION,
    )
    assert len(Struct.__inherited_by__) == len(StructType) - 1
