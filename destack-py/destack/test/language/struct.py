from destack.language import Session, Struct, StructType


def test_struct_inheritance(session: Session):
    """Test the Struct inheritance hierarchy."""
    assert Struct.metatype == StructType.STRUCT
    assert Struct.__is_abstract__
