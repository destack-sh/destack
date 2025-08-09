from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    NodeType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Anchor, Offset2, Quaternion, Vector2, Vector3


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
    rotation: "Vector2" = declare_property(
        112,
        tags=("transform",),
        description="The rotation of the Entity in 2D space.",
    )
    origin: Optional["Vector2"] = declare_property(
        113,
        tags=("transform",),
        description="The origin of the Entity in 2D space.",
    )
    anchor: Optional["Anchor"] = declare_property(
        114,
        tags=("transform",),
        description="The anchor of the Entity in 2D space.",
    )
    offset: Optional["Offset2"] = declare_property(
        115,
        tags=("transform",),
        description="The offset of the Entity in 2D space.",
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
    origin: Optional["Vector3"] = declare_property(
        113,
        tags=("transform",),
        description="The origin of the Entity in 3D space.",
    )
    anchor: Optional["Anchor"] = declare_property(
        114,
        tags=("transform",),
        description="The anchor of the Entity in 3D space.",
    )
