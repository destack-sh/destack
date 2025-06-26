from destack.language.core import (
    StructFrozen,
    StructType,
    Vector2,
    Vector3,
    builtin_struct,
    property_,
)


@builtin_struct(StructType.STROKE_POINT, frozen=True)
class StrokePoint(StructFrozen):
    """A point in a stroke."""

    point: Vector2 = property_(50, is_repr=True)
    input: Vector3 = property_(51, is_repr=True)
    pressure: float = property_(52, is_repr=True)
    vector: Vector3 = property_(53, is_repr=True)
    distance: float = property_(54, is_repr=True)
    running_length: float = property_(55, is_repr=True)
    radius: float = property_(56, is_repr=True)
