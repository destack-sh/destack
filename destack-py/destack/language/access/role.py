from typing import Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    IsDeletable,
    IsJoinable,
    IsOrdered,
    IsTemplatable,
    Node,
    NodeType,
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


@builtin_enum(EnumType.ROLE_TYPE)
class RoleType(Enum):
    """The role of a Role"""

    ADMIN = 1
    DEVELOPER = 4
    USER = 7
    SPECTATOR = 10


@builtin_node(NodeType.ROLE)
class Role(
    Global,
    Entity,
    HasSlug,
    HasIcon,
    HasName,
    IsTemplatable,
    IsOrdered,
    IsDeletable,
    Node[RoleData],
):
    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    type: RoleType = property_(30, is_repr=True)
