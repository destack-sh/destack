from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    Float32,
    NodeType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Quaternion, Vector2, Vector3


@declare_entity(
    NodeType.ENTITY2D,
    is_abstract=True,
)
class Entity2D(Entity):
    """An Entity in 2D space."""

    # transform
    position: "Vector2" = declare_property(
        110,
        tags=("transform",),
        description="The position of the Entity in 2D space.",
    )
    scale: "Vector2" = declare_property(
        111,
        tags=("transform",),
        description="The scale of the Entity in 2D space.",
    )
    rotation: Float32 = declare_property(
        112,
        tags=("transform",),
        default=0.0,
        description="The rotation of the Entity in 2D space.",
    )


@declare_entity(
    NodeType.ENTITY3D,
    is_abstract=True,
)
class Entity3D(Entity):
    """An Entity in 3D space."""

    # transform
    position: "Vector3" = declare_property(
        110,
        tags=("transform",),
        description="The position of the Entity in 3D space.",
    )
    scale: "Vector3" = declare_property(
        111,
        tags=("transform",),
        description="The scale of the Entity in 3D space.",
    )
    rotation: "Quaternion" = declare_property(
        112,
        tags=("transform",),
        description="The rotation of the Entity in 3D space.",
    )
