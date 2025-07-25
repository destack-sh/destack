from destack.language import (
    BasicType,
    PropertyDefinition,
    Session,
    Struct,
    StructType,
    Type,
)


def test_struct_inheritance(session: Session):
    """Test the Struct inheritance hierarchy."""
    # test Struct as base
    assert Struct.metatype == StructType.STRUCT
    assert Struct.__is_abstract__

    # test BasicType extends Struct
    assert BasicType.metatype == StructType.BASIC_TYPE
    assert not BasicType.__is_abstract__
    assert BasicType.__base_type__ == StructType.STRUCT
    assert BasicType.__inherits__ == (StructType.STRUCT,)

    # test Type extends BasicType
    assert Type.metatype == StructType.TYPE
    assert not Type.__is_abstract__
    assert Type.__base_type__ == StructType.BASIC_TYPE
    assert Type.__inherits__ == (StructType.STRUCT, StructType.BASIC_TYPE)

    # test PropertyDefinition extends Type
    assert PropertyDefinition.metatype == StructType.PROPERTY_DEFINITION
    assert not PropertyDefinition.__is_abstract__
    assert PropertyDefinition.__base_type__ == StructType.TYPE
    assert PropertyDefinition.__inherits__ == (
        StructType.STRUCT,
        StructType.BASIC_TYPE,
        StructType.TYPE,
    )
