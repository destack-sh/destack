from typing import TYPE_CHECKING, final

from destack.core import (
    Float32,
    NodeType,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Form3D, Shape3D

if TYPE_CHECKING:
    pass


@declare_struct(
    StructType.CONE3D,
    is_final=True,
    into_node_types=(NodeType.CONE_SHAPE3D,),
)
@final
class Cone3D(Form3D):
    """A Cone is aligned with the local z axis with a base radius and height."""

    radius: Float32 = declare_property(230, is_repr=True, tag=None)
    height: Float32 = declare_property(240, is_repr=True, tag=None)


@declare_entity(
    NodeType.CONE_SHAPE3D,
    base_struct_type=StructType.CONE3D,
)
class ConeShape3D(Shape3D):
    """A ConeShape3D is a 3D shape that represents a cone."""

    radius: Float32 = declare_property(230, is_repr=True, tag=None)
    height: Float32 = declare_property(240, is_repr=True, tag=None)
