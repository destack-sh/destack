from typing import Optional

from destack.language.core import (
    Enum,
    EnumType,
    Event,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    Instance,
    IsDeletable,
    IsJoinable,
    IsOrdered,
    IsOwner,
    Node,
    NodeType,
    RoleType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import RoleData, RoleEventData

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ROLE_EVENT_TYPE)
class RoleEventType(Enum):
    """A Type of Role Event."""

    ASSIGNED = 1, "Assigned", "Assigned to someone", "fas fa-circle"
    REMOVED = 2, "Removed", "Removed from someone", "fas fa-circle"


@builtin_node(NodeType.ROLE_EVENT)
class RoleEvent(
    Event["Role"],
    Node[RoleEventData],
):
    """A Event regarding a Role."""

    type: RoleEventType = property_(30)
    node: "Role" = property_(35)


@builtin_node(NodeType.ROLE)
class Role(
    Global,
    Spatial,
    Instance,
    HasSlug,
    HasIcon,
    HasName,
    IsOwner,
    IsOrdered,
    IsDeletable,
    Node[RoleData],
):
    """A Role for Subjects to take."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    type: RoleType = property_(30, is_repr=True)
