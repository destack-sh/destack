from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    IsDeletable,
    IsGlobal,
    IsJoinable,
    IsOwnable,
    IsSpatial,
    IsSubject,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.INVITE_EVENT, frozen=True, is_abstract=True)
class InviteEvent(Event["Invite"]):
    """A Event regarding an Invite."""

    node: "Invite" = builtin_property(101)
    joinable: "IsJoinable" = builtin_property(102)
    member: "IsSubject" = builtin_property(103)


@builtin_node(NodeType.INVITE_SENT_EVENT, frozen=True)
class InviteSentEvent(InviteEvent):
    """An Invite was sent."""

    role: "Role" = builtin_property(110)
    role_type: "RoleType" = builtin_property(111)


@builtin_node(NodeType.INVITE_RESCINDED_EVENT, frozen=True)
class InviteRescindedEvent(InviteEvent):
    """An Invite was rescinded."""

    pass


@builtin_node(NodeType.INVITE_ACCEPTED_EVENT, frozen=True)
class InviteAcceptedEvent(InviteEvent):
    """An Invite was accepted."""

    role: "Role" = builtin_property(110)
    role_type: "RoleType" = builtin_property(111)


@builtin_node(NodeType.INVITE_REJECTED_EVENT, frozen=True)
class InviteRejectedEvent(InviteEvent):
    """An Invite was rejected."""

    pass


@builtin_node(
    NodeType.INVITE,
    event_types=(NodeType.INVITE_EVENT,),
)
class Invite(IsGlobal, IsSpatial, IsOwnable, IsDeletable, Entity):
    """An Invite to a Joinable."""

    parent: Optional["IsJoinable"] = builtin_property_parent()
    member: "IsSubject" = builtin_property(110)
    role: Optional["Role"] = builtin_property(111)
    role_type: Optional["RoleType"] = builtin_property(112)
