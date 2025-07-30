from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.INVITE_EVENT, is_abstract=True)
class InviteEvent(Event["Invite"]):
    """A Event regarding an Invite."""

    node: "Invite" = builtin_property(101)
    joinable: "Entity" = builtin_property(102)
    member: "Entity" = builtin_property(103)


@builtin_event(NodeType.INVITE_SENT_EVENT)
class InviteSentEvent(InviteEvent):
    """An Invite was sent."""

    role: "Role" = builtin_property(110)
    role_type: "RoleType" = builtin_property(111)


@builtin_event(NodeType.INVITE_RESCINDED_EVENT)
class InviteRescindedEvent(InviteEvent):
    """An Invite was rescinded."""

    pass


@builtin_event(NodeType.INVITE_ACCEPTED_EVENT)
class InviteAcceptedEvent(InviteEvent):
    """An Invite was accepted."""

    role: "Role" = builtin_property(110)
    role_type: "RoleType" = builtin_property(111)


@builtin_event(NodeType.INVITE_REJECTED_EVENT)
class InviteRejectedEvent(InviteEvent):
    """An Invite was rejected."""

    pass


@builtin_entity(
    NodeType.INVITE,
    event_types=(NodeType.INVITE_EVENT,),
    traits=(TraitType.OWNABLE,),
)
class Invite(Entity):
    """An Invite to a Joinable."""

    member: "Entity" = builtin_property(110)
    role: Optional["Role"] = builtin_property(111)
    role_type: Optional["RoleType"] = builtin_property(112)
