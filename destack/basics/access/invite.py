from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    Event,
    NodeType,
    TraitType,
    declare_entity,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@declare_event(NodeType.INVITE_EVENT, is_abstract=True)
class InviteEvent(Event):
    """A Event regarding an Invite."""

    invite: "Invite" = declare_property(101)
    joinable: "Entity" = declare_property(102)
    member: "Entity" = declare_property(103)


@declare_event(NodeType.INVITE_SENT_EVENT)
class InviteSentEvent(InviteEvent):
    """An Invite was sent."""

    role: "Role" = declare_property(110)
    role_type: "RoleType" = declare_property(111)


@declare_event(NodeType.INVITE_RESCINDED_EVENT)
class InviteRescindedEvent(InviteEvent):
    """An Invite was rescinded."""

    pass


@declare_event(NodeType.INVITE_ACCEPTED_EVENT)
class InviteAcceptedEvent(InviteEvent):
    """An Invite was accepted."""

    role: "Role" = declare_property(110)
    role_type: "RoleType" = declare_property(111)


@declare_event(NodeType.INVITE_REJECTED_EVENT)
class InviteRejectedEvent(InviteEvent):
    """An Invite was rejected."""

    pass


@declare_entity(
    NodeType.INVITE,
    event_types=(NodeType.INVITE_EVENT,),
    traits=(TraitType.OWNABLE,),
)
class Invite(Entity):
    """An Invite to a Joinable."""

    member: "Entity" = declare_property(110)
    role: Optional["Role"] = declare_property(111)
    role_type: Optional["RoleType"] = declare_property(112)
