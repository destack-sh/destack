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
    StructType.CYLINDER3D,
    is_final=True,
    into_node_types=(NodeType.CYLINDER_SHAPE3D,),
)
@final
class Cylinder3D(Form3D):
    """A Cylinder is aligned with the local z axis with a radius and height."""

    radius: Float32 = declare_property(210, is_repr=True, tag=None)
    height: Float32 = declare_property(220, is_repr=True, tag=None)


@declare_entity(
    NodeType.CYLINDER_SHAPE3D,
    base_struct_type=StructType.CYLINDER3D,
)
class CylinderShape3D(Shape3D):
    """A CylinderShape3D is a shape that represents a cylinder."""

    radius: Float32 = declare_property(210, is_repr=True, tag=None)
    height: Float32 = declare_property(220, is_repr=True, tag=None)
