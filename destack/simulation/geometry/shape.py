from typing import TYPE_CHECKING, Optional

from destack.core import (
    NodeType,
    StructFrozen,
    StructType,
    builtin_entity,
    builtin_property,
    builtin_struct,
)

from .entity import Entity2D, Entity3D

if TYPE_CHECKING:
    from destack import Stroke


@builtin_struct(
    StructType.FORM2D,
    frozen=True,
    is_abstract=True,
)
class Form2D(StructFrozen):
    """A Form2D represents 2-dimensional geometric Shapes in the abstract."""

    pass


@builtin_entity(
    NodeType.SHAPE2D,
    is_abstract=True,
)
class Shape2D(Entity2D):
    """A Shape2D represents 2-dimensional geometric Shapes situated in space."""

    stroke: Optional["Stroke"] = builtin_property(
        180,
        is_repr=True,
        tags=("style",),
    )


@builtin_struct(
    StructType.FORM3D,
    frozen=True,
    is_abstract=True,
)
class Form3D(StructFrozen):
    """A Form3D represents 3-dimensional geometric Shapes in the abstract."""

    pass


@builtin_entity(
    NodeType.SHAPE3D,
    is_abstract=True,
)
class Shape3D(Entity3D):
    """A Shape3D represents 3-dimensional geometric Shapes situated in space."""

    stroke: Optional["Stroke"] = builtin_property(
        180,
        is_repr=True,
        tags=("style",),
    )
