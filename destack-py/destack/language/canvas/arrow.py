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
from destack.proto import ArrowShapeData

from ..view import ContentView
from .shape import IsShape

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ARROW_HEAD_TYPE)
class ArrowHeadType(Enum):
    ARROW = 1
    TRIANGLE = 2
    DOT = 3


@builtin_node(NodeType.ARROW_SHAPE, pretend_frozen=True)
class ArrowShape(ContentView, IsShape, Node[ArrowShapeData]):
    """An ArrowShape is a shape that represents an arrow."""

    # content
    start_type: ArrowHeadType = property_(100)
    start: Vector2 = property_(101)
    end_type: ArrowHeadType = property_(110)
    end: Vector2 = property_(111)
