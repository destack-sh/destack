from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    IsActor,
    IsDeletable,
    IsExtensible,
    IsJoinable,
    IsOrdered,
    NodeType,
    RoleType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ROLE_EVENT, frozen=True, is_abstract=True)
class RoleEvent(Event["Role"]):
    """A Event regarding a Role."""

    node: "Role" = builtin_property(101)
    actor: "IsActor" = builtin_property(110)


@builtin_node(NodeType.ROLE_ASSIGNED_EVENT, frozen=True)
class RoleAssignedEvent(RoleEvent):
    """A Event regarding a Role."""

    pass


@builtin_node(NodeType.ROLE_UNASSIGNED_EVENT, frozen=True)
class RoleUnassignedEvent(RoleEvent):
    """A Event regarding a Role."""

    pass


@builtin_node(
    NodeType.ROLE,
    event_types=(NodeType.ROLE_EVENT,),
)
class Role(
    IsActor,
    IsOrdered,
    IsDeletable,
    IsExtensible,
    Entity,
):
    """A Role for Actors to take."""

    parent: Optional["IsJoinable"] = builtin_property_parent()
    type: RoleType = builtin_property(100, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
