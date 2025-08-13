from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    Float32,
    NodeType,
    TagDeclaration,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Quaternion, Vector2, Vector3


@declare_entity(
    NodeType.ENTITY2D,
    is_abstract=True,
    event_types=(NodeType.POINTER_EVENT,),
    tags=(TagDeclaration(id=110, name="transform", description="Transform"),),
)
class Entity2D(Entity):
    """An Entity in 2D space."""

    # transform
    position: "Vector2" = declare_property(
        110,
        description="The position of the Entity in 2D space.",
        tag="transform",
    )
    scale: "Vector2" = declare_property(
        111,
        description="The scale of the Entity in 2D space.",
        tag="transform",
    )
    rotation: Float32 = declare_property(
        112,
        default=0.0,
        description="The rotation of the Entity in 2D space.",
        tag="transform",
    )


@declare_entity(
    NodeType.ENTITY3D,
    is_abstract=True,
    tags=(TagDeclaration(id=110, name="transform", description="Transform"),),
)
class Entity3D(Entity):
    """An Entity in 3D space."""

    # transform
    position: "Vector3" = declare_property(
        110,
        description="The position of the Entity in 3D space.",
        tag="transform",
    )
    scale: "Vector3" = declare_property(
        111,
        description="The scale of the Entity in 3D space.",
        tag="transform",
    )
    rotation: "Quaternion" = declare_property(
        112,
        description="The rotation of the Entity in 3D space.",
        tag="transform",
    )
