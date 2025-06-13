from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinEnum,
    Entity,
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
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import MembershipData

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.MEMBERSHIP_EVENT_TYPE)
class MembershipEventType(BuiltinEnum):
    """A Type of Membership Event."""

    JOIN = 1, "Join", "Join", "fas fa-user-plus"
    LEAVE = 2, "Leave", "Leave", "fas fa-user-minus"


@node_(NodeType.MEMBERSHIP_EVENT)
class MembershipEvent(
    Spatial,
    Event,
    Node["MembershipEventData"],
):
    """A Event regarding a Membership."""


@node_(NodeType.MEMBERSHIP)
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
