from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    Event,
    NodeType,
    RoleType,
    TraitType,
    declare_entity,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Icon

# pyright: reportIncompatibleVariableOverride=false


@declare_event(NodeType.ROLE_EVENT, is_abstract=True)
class RoleEvent(Event):
    """A Event regarding a Role."""

    role: "Role" = declare_property(101)
    actor: "Entity" = declare_property(110)


@declare_event(NodeType.ROLE_ASSIGNED_EVENT)
class RoleAssignedEvent(RoleEvent):
    """A Event regarding a Role."""

    pass


@declare_event(NodeType.ROLE_UNASSIGNED_EVENT)
class RoleUnassignedEvent(RoleEvent):
    """A Event regarding a Role."""

    pass


@declare_entity(
    NodeType.ROLE,
    event_types=(NodeType.ROLE_EVENT,),
    traits=(TraitType.OWNABLE,),
)
class Role(Entity):
    """A Role for Actors to take."""

    type: RoleType = declare_property(100, is_repr=True)
    icon: "Icon | None" = declare_property(102)
