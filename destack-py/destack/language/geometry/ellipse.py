from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    NodeType,
    StructFrozen,
    StructType,
    builtin_entity,
    builtin_property,
    builtin_struct,
)

from .shape import Shape2D

if TYPE_CHECKING:
    from destack.language import Stroke

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.ELLIPSE2D, frozen=True)
class Ellipse2D(StructFrozen):
    """A Ellipse is a circle."""

    stroke: Optional["Stroke"] = builtin_property(200, is_repr=True)


@builtin_entity(NodeType.ELLIPSE_SHAPE2D)
class EllipseShape2D(Shape2D):
    """A EllipseShape is a shape that represents a ellipse."""

    pass
