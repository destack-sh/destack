from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    Global,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsSubject,
    IsTemplatable,
    LikeMembership,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import MembershipData, MembershipEventData

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.MEMBERSHIP_EVENT_TYPE)
class MembershipEventType(Enum):
    """A Type of Membership Event."""

    JOIN = 1, "Join", "Join", "fas fa-user-plus"
    LEAVE = 2, "Leave", "Leave", "fas fa-user-minus"


@builtin_node(NodeType.MEMBERSHIP_EVENT)
class MembershipEvent(
    Event,
    Node[MembershipEventData],
):
    """A Event regarding a Membership."""


@builtin_node(NodeType.MEMBERSHIP)
class Membership(
    Global,
    Spatial,
    Entity,
    LikeMembership,
    IsTemplatable,
    IsOwnable,
    IsDeletable,
    Node[MembershipData],
):
    """A Membership of a Subject in a Joinable."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    member: Optional["IsSubject"] = property_(40)
    role: Optional["Role"] = property_(41)
    role_type: Optional["RoleType"] = property_(42)
