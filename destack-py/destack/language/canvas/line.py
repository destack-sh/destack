from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    NodeType,
    StructFrozen,
    StructType,
    Vector2f,
    builtin_node,
    builtin_property,
    builtin_struct,
)
from destack.proto import LineProto

from .shape import Shape

if TYPE_CHECKING:
    from destack.language import Stroke

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.LINE, frozen=True)
class Line(StructFrozen[LineProto]):
    """A Line is a list of points."""

    stroke: Optional["Stroke"] = builtin_property(200, is_repr=True)
    points: list[Vector2f] = builtin_property(210)


@builtin_node(NodeType.LINE_SHAPE, pretend_frozen=True)
class LineShape(Shape):
    """A LineShape is a shape that represents a line."""

    points: list[Vector2f] = builtin_property(200)
