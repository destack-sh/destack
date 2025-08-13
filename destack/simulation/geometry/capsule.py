from typing import TYPE_CHECKING, final

from destack.core import (
    Float32,
    NodeType,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Form3D, Shape2D, Shape3D
from .vector import Vector2, Vector3

if TYPE_CHECKING:
    from destack import Vector2


@declare_struct(
    StructType.CAPSULE2D,
    is_final=True,
    into_node_types=(NodeType.CAPSULE_SHAPE2D,),
)
@final
class Capsule2D(Form2D):
    """A Capsule2D is a segment with rounded ends with a common radius."""

    center_a: "Vector2" = declare_property(210, tag=None)
    center_b: "Vector2" = declare_property(211, tag=None)
    radius: Float32 = declare_property(212, tag=None)


@declare_entity(
    NodeType.CAPSULE_SHAPE2D,
    base_struct_type=StructType.CAPSULE2D,
)
class CapsuleShape2D(Shape2D):
    """A CapsuleShape is a shape that represents a capsule."""

    center_a: "Vector2" = declare_property(210, tag=None)
    center_b: "Vector2" = declare_property(211, tag=None)
    radius: Float32 = declare_property(212, tag=None)


@declare_struct(
    StructType.CAPSULE3D,
    is_final=True,
    into_node_types=(NodeType.CAPSULE_SHAPE3D,),
)
@final
class Capsule3D(Form3D):
    """A Capsule3D is a segment with rounded ends in 3D with a common radius."""

    center_a: "Vector3" = declare_property(210, tag=None)
    center_b: "Vector3" = declare_property(211, tag=None)
    radius: Float32 = declare_property(212, tag=None)


@declare_entity(
    NodeType.CAPSULE_SHAPE3D,
    base_struct_type=StructType.CAPSULE3D,
)
class CapsuleShape3D(Shape3D):
    """A CapsuleShape3D is a shape that represents a capsule in 3D."""

    center_a: "Vector3" = declare_property(210, tag=None)
    center_b: "Vector3" = declare_property(211, tag=None)
    radius: Float32 = declare_property(212, tag=None)
