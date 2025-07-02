from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    IsDeletable,
    IsGlobal,
    IsJoinable,
    IsOrdered,
    IsOwner,
    IsSpatial,
    IsSubject,
    NodeType,
    RoleType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ROLE_EVENT, is_abstract=True)
class RoleEvent(Event["Role"]):
    """A Event regarding a Role."""

    subject: "IsSubject" = builtin_property(40)


@builtin_node(NodeType.ROLE_ASSIGNED_EVENT)
class RoleAssignedEvent(RoleEvent):
    """A Event regarding a Role."""

    pass


@builtin_node(NodeType.ROLE_UNASSIGNED_EVENT)
class RoleUnassignedEvent(RoleEvent):
    """A Event regarding a Role."""

    pass


@builtin_node(NodeType.ROLE)
class Role(
    IsGlobal,
    IsSpatial,
    IsOwner,
    IsOrdered,
    IsDeletable,
    Entity,
):
    """A Role for Subjects to take."""

    parent: Optional["IsJoinable"] = builtin_property_parent(node_is_extensible=False)
    type: RoleType = builtin_property(100, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
