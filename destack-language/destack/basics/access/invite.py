from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    Event,
    NodeType,
    ReferenceType,
    TraitType,
    declare_entity,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Role, RoleType


@declare_event(NodeType.INVITE_EVENT, is_abstract=True)
class InviteEvent(Event):
    """A Event regarding an Invite."""

    invite: "Invite" = declare_property(
        101,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    joinable: "Entity" = declare_property(
        102,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    member: "Entity" = declare_property(
        103,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )


@declare_event(NodeType.INVITE_SENT_EVENT)
class InviteSentEvent(InviteEvent):
    """An Invite was sent."""

    role: "Role" = declare_property(
        110,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    role_type: "RoleType" = declare_property(
        111,
        tag=None,
    )


@declare_event(NodeType.INVITE_RESCINDED_EVENT)
class InviteRescindedEvent(InviteEvent):
    """An Invite was rescinded."""

    pass


@declare_event(NodeType.INVITE_ACCEPTED_EVENT)
class InviteAcceptedEvent(InviteEvent):
    """An Invite was accepted."""

    role: "Role" = declare_property(
        110,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    role_type: "RoleType" = declare_property(
        111,
        tag=None,
    )


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

    member: "Entity" = declare_property(
        110,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    role: Optional["Role"] = declare_property(
        111,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    role_type: Optional["RoleType"] = declare_property(112, tag=None)
