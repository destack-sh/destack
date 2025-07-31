from typing import TYPE_CHECKING, final

from destack.language.core import (
    NodeType,
    StructType,
    builtin_entity,
    builtin_property,
    builtin_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(
    StructType.LINE2D,
    frozen=True,
    is_final=True,
)
@final
class Line2D(Form2D):
    """A Line is a line between two points."""

    start: "Vector2" = builtin_property(210)
    end: "Vector2" = builtin_property(220)


@builtin_entity(NodeType.LINE_SHAPE2D)
class LineShape2D(Shape2D):
    """A LineShape is a shape that represents a line between two points."""

    start: "Vector2" = builtin_property(200)
    end: "Vector2" = builtin_property(210)
