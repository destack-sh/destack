from typing import Optional

from destack.language.core import (
    Entity,
    Event,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    IsDeletable,
    IsJoinable,
    IsOrdered,
    IsOwner,
    IsSubject,
    Node,
    NodeType,
    RoleType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ROLE_ASSIGNED_EVENT)
class RoleAssignedEvent(
    Event["Role"],
    Node,
):
    """A Event regarding a Role."""

    subject: "IsSubject" = property_(40)


@builtin_node(NodeType.ROLE_UNASSIGNED_EVENT)
class RoleUnassignedEvent(
    Event["Role"],
    Node,
):
    """A Event regarding a Role."""

    subject: "IsSubject" = property_(40)


@builtin_node(NodeType.ROLE)
class Role(
    Global,
    Spatial,
    Entity,
    HasSlug,
    HasIcon,
    HasName,
    IsOwner,
    IsOrdered,
    IsDeletable,
    Node,
):
    """A Role for Subjects to take."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    type: RoleType = property_(30, is_repr=True)
