from destack.language.core import (
    BuiltinEnum,
    EnumType,
    Node,
    NodeType,
    Vector2,
    enum_,
    node_,
    property_,
)
from destack.pb2 import PlaneShapeData

from ..view import ContainerView
from .shape import IsShape

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PLANE_SHAPE_TYPE)
class PlaneShapeType(BuiltinEnum):
    RECTANGLE = 1
    TRIANGLE = 2
    CIRCLE = 3
    ELLIPSE = 4
    POLYGON = 5


@node_(NodeType.PLANE_SHAPE, pretend_frozen=True)
class PlaneShape(ContainerView, IsShape, Node[PlaneShapeData]):
    points: list[Vector2] = property_(100)
