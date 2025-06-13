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
    Node,
    NodeType,
    Spatial,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import InviteData

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.INVITE_EVENT_TYPE)
class InviteEventType(BuiltinEnum):
    """A Type of Invite Event."""

    SENT = 1, "Sent", "Sent", "fas fa-envelope"
    RESCINDED = 2, "Rescinded", "Rescinded", "fas fa-times"
    ACCEPTED = 3, "Accepted", "Accepted", "fas fa-check"
    REJECTED = 4, "Rejected", "Rejected", "fas fa-times"


@node_(NodeType.INVITE_EVENT)
class InviteEvent(
    Spatial,
    Event,
    Node["InviteEventData"],
):
    """A Event regarding an Invite."""


@node_(NodeType.INVITE)
class Invite(
    Global,
    Spatial,
    Entity,
    IsTemplatable,
    IsOwnable,
    IsDeletable,
    Node[InviteData],
):
    """An Invite to a Joinable."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    member: Optional["IsSubject"] = property_(40)
    role: Optional["Role"] = property_(41)
    role_type: Optional["RoleType"] = property_(42)
