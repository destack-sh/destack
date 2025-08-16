from destack import (
    Form2D,
    Rectangle2D,
    Struct,
    StructType,
)


def test_struct_inheritance():
    """Test the Struct inheritance hierarchy."""
    # test Struct as base
    assert Struct.metatype == StructType.STRUCT
    assert Struct.__definition__.is_abstract

    # test Form2D extends Struct
    assert Form2D.metatype == StructType.FORM2D
    assert Form2D.__definition__.is_abstract
    assert Form2D.__definition__.base_type == StructType.STRUCT
    assert Form2D.__definition__.inherits == [StructType.STRUCT]

    # test Rectangle2D extends Form2D
    assert Rectangle2D.metatype == StructType.RECTANGLE2D
    assert not Rectangle2D.__definition__.is_abstract
    assert Rectangle2D.__definition__.base_type == StructType.FORM2D
    assert Rectangle2D.__definition__.inherits == [
        StructType.STRUCT,
        StructType.FORM2D,
    ]
