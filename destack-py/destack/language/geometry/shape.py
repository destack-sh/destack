from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity2D,
    Entity3D,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Stroke


@builtin_node(
    NodeType.SHAPE2D,
    is_abstract=True,
)
class Shape2D(Entity2D):
    """A Shape2D represents 2-dimensional geometric Shapes."""

    stroke: Optional["Stroke"] = builtin_property(
        180,
        is_repr=True,
        tags=("style",),
    )


@builtin_node(
    NodeType.SHAPE3D,
    is_abstract=True,
)
class Shape3D(Entity3D):
    """A Shape3D represents 3-dimensional geometric Shapes."""

    stroke: Optional["Stroke"] = builtin_property(
        180,
        is_repr=True,
        tags=("style",),
    )
