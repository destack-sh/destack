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
    assert Struct.__definition__.is_abstract

    # test Type extends Struct
    assert Type.metatype == StructType.TYPE
    assert not Type.__definition__.is_abstract
    assert Type.__definition__.base_type == StructType.STRUCT
    assert Type.__definition__.inherits == [StructType.STRUCT]

    # test CheckedType extends Type
    assert CheckedType.metatype == StructType.CHECKED_TYPE
    assert not CheckedType.__definition__.is_abstract
    assert CheckedType.__definition__.base_type == StructType.TYPE
    assert CheckedType.__definition__.inherits == [StructType.STRUCT, StructType.TYPE]

    # test PropertyDefinition extends CheckedType
    assert PropertyDefinition.metatype == StructType.PROPERTY_DEFINITION
    assert not PropertyDefinition.__definition__.is_abstract
    assert PropertyDefinition.__definition__.base_type == StructType.CHECKED_TYPE
    assert PropertyDefinition.__definition__.inherits == [
        StructType.STRUCT,
        StructType.TYPE,
        StructType.CHECKED_TYPE,
    ]
