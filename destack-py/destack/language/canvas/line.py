from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    NodeType,
    StructFrozen,
    StructType,
    builtin_node,
    builtin_property,
    builtin_struct,
)
from destack.proto import LineProto

from .shape import Shape

if TYPE_CHECKING:
    from destack.language import Stroke, Vector2f

# pyright: reportIncompatibleVariableOverride=false


# nocheckin: consolidate geometry stuff? (into geometry category)


@builtin_struct(StructType.LINE, frozen=True)
class Line(StructFrozen[LineProto]):
    """A Line is a list of points."""

    stroke: Optional["Stroke"] = builtin_property(200, is_repr=True)
    points: list["Vector2f"] = builtin_property(210)


@builtin_node(NodeType.LINE_SHAPE)
class LineShape(Shape):
    """A LineShape is a shape that represents a line."""

    points: list["Vector2f"] = builtin_property(200)
