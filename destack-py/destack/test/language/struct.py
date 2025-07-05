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
        StructType.TRAIT_DEFINITION,
        StructType.NODE_DEFINITION,
        StructType.STRUCT_DEFINITION,
    )
    assert len(Struct.__inherited_by__) == len(StructType) - 1
