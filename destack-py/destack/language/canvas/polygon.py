from typing import TYPE_CHECKING

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    Vector2,
    builtin_enum,
    builtin_node,
    builtin_struct,
    property_,
)
from destack.proto import PolygonProto

from .shape import Shape

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.POLYGON_SHAPE_TYPE)
class PolygonShapeType(Enum):
    RECTANGLE = 1
    TRIANGLE = 2
    CIRCLE = 3
    ELLIPSE = 4
    POLYGON = 5


@builtin_struct(StructType.POLYGON, frozen=True)
class Polygon(StructFrozen[PolygonProto]):
    """A Polygon is a list of points."""

    type: PolygonShapeType = property_(30, is_repr=True)
    points: list[Vector2] = property_(100)


@builtin_node(NodeType.POLYGON_SHAPE, pretend_frozen=True)
class PolygonShape(Shape):
    """A PolygonShape is a shape that represents a polygon."""

    type: PolygonShapeType = property_(30, is_repr=True)
    points: list[Vector2] = property_(100)
