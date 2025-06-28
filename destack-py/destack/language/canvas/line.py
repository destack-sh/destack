from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Node,
    NodeType,
    StructFrozen,
    StructType,
    Vector3,
    builtin_node,
    builtin_struct,
    property_,
)
from destack.proto import LineProto, LineShapeProto

from ..view import ContentView
from .shape import IsShape

if TYPE_CHECKING:
    from destack.language import Stroke

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.LINE, frozen=True)
class Line(StructFrozen[LineProto]):
    """A Line is a list of points."""

    points: list[Vector3] = property_(100)
    stroke: Optional["Stroke"] = property_(101, is_repr=True)


@builtin_node(NodeType.LINE_SHAPE, pretend_frozen=True)
class LineShape(ContentView, IsShape, Node[LineShapeProto]):
    """A LineShape is a shape that represents a line."""

    points: list[Vector3] = property_(100)
    stroke: Optional["Stroke"] = property_(101, is_repr=True)
