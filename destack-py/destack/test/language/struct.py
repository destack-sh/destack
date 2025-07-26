from destack.language import (
    CheckedType,
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

    # test Type extends Struct
    assert Type.metatype == StructType.TYPE
    assert not Type.__is_abstract__
    assert Type.__base_type__ == StructType.STRUCT
    assert Type.__inherits__ == (StructType.STRUCT,)

    # test CheckedType extends Type
    assert CheckedType.metatype == StructType.CHECKED_TYPE
    assert not CheckedType.__is_abstract__
    assert CheckedType.__base_type__ == StructType.TYPE
    assert CheckedType.__inherits__ == (StructType.STRUCT, StructType.TYPE)

    # test PropertyDefinition extends CheckedType
    assert PropertyDefinition.metatype == StructType.PROPERTY_DEFINITION
    assert not PropertyDefinition.__is_abstract__
    assert PropertyDefinition.__base_type__ == StructType.CHECKED_TYPE
    assert PropertyDefinition.__inherits__ == (
        StructType.STRUCT,
        StructType.TYPE,
        StructType.CHECKED_TYPE,
    )
