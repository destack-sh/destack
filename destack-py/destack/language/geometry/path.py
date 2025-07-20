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


@builtin_struct(StructType.PATH2D, frozen=True)
class Path2D(StructFrozen):
    """A Path is a path of multiple points."""

    stroke: Optional["Stroke"] = builtin_property(200, is_repr=True)
    points: list["Vector2"] = builtin_property(210)


@builtin_node(NodeType.PATH_SHAPE2D)
class PathShape2D(Shape2D):
    """A PathShape is a shape that represents a path of multiple points."""

    points: list["Vector2"] = builtin_property(200)
