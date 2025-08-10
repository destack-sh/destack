from typing import TYPE_CHECKING

from destack.core import (
    ImmutableStruct,
    NodeType,
    StructType,
    declare_entity,
    declare_struct,
)

from .entity import Entity2D, Entity3D

if TYPE_CHECKING:
    pass


@declare_struct(
    StructType.FORM2D,
    frozen=True,
    is_abstract=True,
)
class Form2D(ImmutableStruct):
    """A Form2D represents 2-dimensional geometric Shapes in the abstract."""

    pass


@declare_entity(
    NodeType.SHAPE2D,
    is_abstract=True,
)
class Shape2D(Entity2D):
    """A Shape2D represents 2-dimensional geometric Shapes situated in space."""

    pass


@declare_struct(
    StructType.FORM3D,
    frozen=True,
    is_abstract=True,
)
class Form3D(ImmutableStruct):
    """A Form3D represents 3-dimensional geometric Shapes in the abstract."""

    pass


@declare_entity(
    NodeType.SHAPE3D,
    is_abstract=True,
)
class Shape3D(Entity3D):
    """A Shape3D represents 3-dimensional geometric Shapes situated in space."""

    pass
