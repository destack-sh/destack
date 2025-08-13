from typing import TYPE_CHECKING

from destack.core import (
    NodeType,
    Struct,
    StructType,
    declare_entity,
    declare_struct,
)

from .entity import Entity2D, Entity3D

if TYPE_CHECKING:
    pass


@declare_struct(
    StructType.FORM2D,
    is_abstract=True,
)
class Form2D(Struct):
    """Represent 2-dimensional geometric Shapes in the abstract."""

    pass


@declare_entity(
    NodeType.SHAPE2D,
    is_abstract=True,
)
class Shape2D(Entity2D):
    """Represent 2-dimensional geometric Shapes situated in space."""

    pass


@declare_struct(
    StructType.FORM3D,
    is_abstract=True,
)
class Form3D(Struct):
    """Represent 3-dimensional geometric Shapes in the abstract."""

    pass


@declare_entity(
    NodeType.SHAPE3D,
    is_abstract=True,
)
class Shape3D(Entity3D):
    """Represent 3-dimensional geometric Shapes situated in space."""

    pass
