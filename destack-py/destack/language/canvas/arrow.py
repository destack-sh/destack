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
from destack.pb2 import ArrowShapeData

from ..view import ContentView
from .shape import IsShape

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ARROW_HEAD_TYPE)
class ArrowHeadType(BuiltinEnum):
    ARROW = 1
    TRIANGLE = 2
    DOT = 3


@node_(NodeType.ARROW_SHAPE, pretend_frozen=True)
class ArrowShape(ContentView, IsShape, Node[ArrowShapeData]):
    """An ArrowShape is a shape that represents an arrow."""

    # content
    start_type: ArrowHeadType = property_(100)
    start: Vector2 = property_(101)
    end_type: ArrowHeadType = property_(110)
    end: Vector2 = property_(111)
