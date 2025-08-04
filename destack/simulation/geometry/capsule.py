from typing import TYPE_CHECKING

from destack.core import (
    Float32,
    NodeType,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack import Vector2


@declare_struct(StructType.CAPSULE2D, frozen=True)
class Capsule2D(Form2D):
    """A Capsule2D is a cylinder with a radius and height."""

    center_a: "Vector2" = declare_property(210)
    center_b: "Vector2" = declare_property(211)
    radius: Float32 = declare_property(212)


@declare_entity(NodeType.CAPSULE_SHAPE2D)
class CapsuleShape2D(Shape2D):
    """A CapsuleShape is a shape that represents a capsule."""

    center_a: "Vector2" = declare_property(210)
    center_b: "Vector2" = declare_property(211)
    radius: Float32 = declare_property(212)
