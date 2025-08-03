from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    NodeType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Anchor, Offset2, Quaternion, Vector2, Vector3

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(
    NodeType.ENTITY2D,
    is_abstract=True,
)
class Entity2D(Entity):
    """An Entity in 2D space."""

    # transform
    position: Optional["Vector2"] = declare_property(
        110,
        tags=("transform",),
    )
    offset: Optional["Offset2"] = declare_property(
        111,
        tags=("transform",),
    )
    scale: Optional["Vector2"] = declare_property(
        112,
        tags=("transform",),
    )
    rotation: Optional["Vector2"] = declare_property(
        113,
        tags=("transform",),
    )
    skew: Optional["Vector2"] = declare_property(
        114,
        tags=("transform",),
    )
    origin: Optional["Vector2"] = declare_property(
        115,
        tags=("transform",),
    )
    anchor: Optional["Anchor"] = declare_property(
        116,
        tags=("transform",),
    )


@declare_entity(
    NodeType.ENTITY3D,
    is_abstract=True,
)
class Entity3D(Entity):
    """An Entity in 3D space."""

    # transform
    position: Optional["Vector3"] = declare_property(
        110,
        tags=("transform",),
    )
    scale: Optional["Vector3"] = declare_property(
        111,
        tags=("transform",),
    )
    rotation: Optional["Quaternion"] = declare_property(
        112,
        tags=("transform",),
    )
    skew: Optional["Vector3"] = declare_property(
        113,
        tags=("transform",),
    )
    origin: Optional["Vector3"] = declare_property(
        114,
        tags=("transform",),
    )
    anchor: Optional["Anchor"] = declare_property(
        115,
        tags=("transform",),
    )
