from typing import TYPE_CHECKING, Optional

from ..builtin import Entity, IsExtensible, NodeType, builtin_node, builtin_property

if TYPE_CHECKING:
    from destack.language import Anchor, Offset2, Quaternion, Vector2, Vector3


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_node(NodeType.ENTITY2D, is_abstract=True)
class Entity2D(
    IsExtensible,
    Entity,
):
    """An Entity in 2D space."""

    # transform
    position: Optional["Vector2"] = builtin_property(110)
    offset: Optional["Offset2"] = builtin_property(111)
    scale: Optional["Vector2"] = builtin_property(112)
    rotation: Optional["Vector2"] = builtin_property(113)
    skew: Optional["Vector2"] = builtin_property(114)
    origin: Optional["Vector2"] = builtin_property(115)
    anchor: Optional["Anchor"] = builtin_property(116)


@builtin_node(NodeType.ENTITY3D, is_abstract=True)
class Entity3D(
    IsExtensible,
    Entity,
):
    """An Entity in 3D space."""

    # transform
    position: Optional["Vector3"] = builtin_property(110)
    scale: Optional["Vector3"] = builtin_property(111)
    rotation: Optional["Quaternion"] = builtin_property(112)
    skew: Optional["Vector3"] = builtin_property(113)
    origin: Optional["Vector3"] = builtin_property(114)
