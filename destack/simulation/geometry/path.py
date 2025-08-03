from typing import TYPE_CHECKING, final

from destack.core import (
    NodeType,
    StructType,
    builtin_entity,
    builtin_property,
    builtin_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(
    StructType.PATH2D,
    frozen=True,
    is_final=True,
)
@final
class Path2D(Form2D):
    """A Path is a path of multiple points."""

    points: list["Vector2"] = builtin_property(210)


@builtin_entity(NodeType.PATH_SHAPE2D)
class PathShape2D(Shape2D):
    """A PathShape is a shape that represents a path of multiple points."""

    points: list["Vector2"] = builtin_property(200)
