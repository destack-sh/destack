from destack.language.core import (
    Enum,
    EnumType,
    Node,
    NodeType,
    Vector2,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.proto import PlaneShapeProto

from ..view import ContainerView
from .shape import IsShape

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.PLANE_SHAPE_TYPE)
class PlaneShapeType(Enum):
    RECTANGLE = 1
    TRIANGLE = 2
    CIRCLE = 3
    ELLIPSE = 4
    POLYGON = 5


@builtin_node(NodeType.PLANE_SHAPE, pretend_frozen=True)
class PlaneShape(ContainerView, IsShape, Node[PlaneShapeProto]):
    points: list[Vector2] = property_(100)
