from typing import TYPE_CHECKING, Optional

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
from destack.pb2 import LineShapeData

from ..view import ContentView
from .shape import IsShape

if TYPE_CHECKING:
    from destack.language import Color

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.LINE_TYPE)
class LineType(BuiltinEnum):
    SOLID = 1
    DASHED = 2
    DOTTED = 3


@node_(NodeType.LINE_SHAPE, pretend_frozen=True)
class LineShape(ContentView, IsShape, Node[LineShapeData]):
    """A LineShape is a shape that represents a line."""

    type: LineType = property_(30)
    points: list[Vector2] = property_(100)
    color: Optional["Color"] = property_(101, is_repr=True)
