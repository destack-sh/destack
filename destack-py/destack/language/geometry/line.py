from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    NodeType,
    StructFrozen,
    StructType,
    builtin_node,
    builtin_property,
    builtin_struct,
)

from .shape import Shape2D

if TYPE_CHECKING:
    from destack.language import Stroke, Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.LINE2D, frozen=True)
class Line2D(StructFrozen):
    """A Line is a line between two points."""

    stroke: Optional["Stroke"] = builtin_property(200, is_repr=True)
    start: "Vector2" = builtin_property(210)
    end: "Vector2" = builtin_property(220)


@builtin_node(NodeType.LINE_SHAPE2D)
class LineShape2D(Shape2D):
    """A LineShape is a shape that represents a line between two points."""

    start: "Vector2" = builtin_property(200)
    end: "Vector2" = builtin_property(210)
