from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    NodeType,
    StructFrozen,
    StructType,
    Vector2,
    builtin_node,
    builtin_struct,
    property_,
)
from destack.proto import LineProto

from .shape import Shape

if TYPE_CHECKING:
    from destack.language import Stroke

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.LINE, frozen=True)
class Line(StructFrozen[LineProto]):
    """A Line is a list of points."""

    stroke: Optional["Stroke"] = property_(80, is_repr=True)
    points: list[Vector2] = property_(100)


@builtin_node(NodeType.LINE_SHAPE, pretend_frozen=True)
class LineShape(Shape):
    """A LineShape is a shape that represents a line."""

    points: list[Vector2] = property_(100)
