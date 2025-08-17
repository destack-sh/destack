from typing import TYPE_CHECKING

from destack.core import (
    NodeType,
    Struct,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .entity import Entity2D, Entity3D

if TYPE_CHECKING:
    from destack import Vector2, Vector3


@declare_struct(
    StructType.FORM2D,
    is_abstract=True,
)
class Form2D(Struct):
    """Represent 2-dimensional geometric shapes with position in the abstract."""

    position: "Vector2" = declare_property(200, is_repr=True, tag=None)


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
    """Represent 3-dimensional geometric shapes with position in the abstract."""

    position: "Vector3" = declare_property(200, is_repr=True, tag=None)


@declare_entity(
    NodeType.SHAPE3D,
    is_abstract=True,
)
class Shape3D(Entity3D):
    """Represent 3-dimensional geometric Shapes situated in space."""

    pass
