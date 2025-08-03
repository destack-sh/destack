from typing import TYPE_CHECKING

from destack.language.core import (
    Float32,
    NodeType,
    StructType,
    builtin_entity,
    builtin_property,
    builtin_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.CAPSULE2D, frozen=True)
class Capsule2D(Form2D):
    """A Capsule2D is a cylinder with a radius and height."""

    center_a: "Vector2" = builtin_property(210)
    center_b: "Vector2" = builtin_property(211)
    radius: Float32 = builtin_property(212)


@builtin_entity(NodeType.CAPSULE_SHAPE2D)
class CapsuleShape2D(Shape2D):
    """A CapsuleShape is a shape that represents a capsule."""

    center_a: "Vector2" = builtin_property(210)
    center_b: "Vector2" = builtin_property(211)
    radius: Float32 = builtin_property(212)
