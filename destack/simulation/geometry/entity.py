from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    NodeType,
    builtin_entity,
    builtin_property,
)

if TYPE_CHECKING:
    from destack import Anchor, Offset2, Quaternion, Vector2, Vector3

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.ENTITY2D,
    is_abstract=True,
)
class Entity2D(Entity):
    """An Entity in 2D space."""

    # transform
    position: Optional["Vector2"] = builtin_property(
        110,
        tags=("transform",),
    )
    offset: Optional["Offset2"] = builtin_property(
        111,
        tags=("transform",),
    )
    scale: Optional["Vector2"] = builtin_property(
        112,
        tags=("transform",),
    )
    rotation: Optional["Vector2"] = builtin_property(
        113,
        tags=("transform",),
    )
    skew: Optional["Vector2"] = builtin_property(
        114,
        tags=("transform",),
    )
    origin: Optional["Vector2"] = builtin_property(
        115,
        tags=("transform",),
    )
    anchor: Optional["Anchor"] = builtin_property(
        116,
        tags=("transform",),
    )


@builtin_entity(
    NodeType.ENTITY3D,
    is_abstract=True,
)
class Entity3D(Entity):
    """An Entity in 3D space."""

    # transform
    position: Optional["Vector3"] = builtin_property(
        110,
        tags=("transform",),
    )
    scale: Optional["Vector3"] = builtin_property(
        111,
        tags=("transform",),
    )
    rotation: Optional["Quaternion"] = builtin_property(
        112,
        tags=("transform",),
    )
    skew: Optional["Vector3"] = builtin_property(
        113,
        tags=("transform",),
    )
    origin: Optional["Vector3"] = builtin_property(
        114,
        tags=("transform",),
    )
    anchor: Optional["Anchor"] = builtin_property(
        115,
        tags=("transform",),
    )
