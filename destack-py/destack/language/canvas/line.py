from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    Node,
    NodeType,
    StructFrozen,
    StructType,
    Vector3,
    builtin_enum,
    builtin_node,
    builtin_struct,
    property_,
)
from destack.proto import LineProto, LineShapeProto

from ..view import ContentView
from .shape import IsShape

if TYPE_CHECKING:
    from destack.language import Color

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LINE_TYPE)
class LineType(Enum):
    SOLID = 1
    DASHED = 2
    DOTTED = 3


@builtin_struct(StructType.LINE, frozen=True)
class Line(StructFrozen[LineProto]):
    """A Line is a list of points."""

    type: LineType = property_(30, is_repr=True)
    points: list[Vector3] = property_(100)
    color: Optional["Color"] = property_(101, is_repr=True)


@builtin_node(NodeType.LINE_SHAPE, pretend_frozen=True)
class LineShape(ContentView, IsShape, Node[LineShapeProto]):
    """A LineShape is a shape that represents a line."""

    type: LineType = property_(30, is_repr=True)
    points: list[Vector3] = property_(100)
    color: Optional["Color"] = property_(101, is_repr=True)
