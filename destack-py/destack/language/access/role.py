from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Event,
    NodeType,
    RoleType,
    TraitType,
    builtin_entity,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.ROLE_EVENT, is_abstract=True)
class RoleEvent(Event):
    """A Event regarding a Role."""

    role: "Role" = builtin_property(101)
    actor: "Entity" = builtin_property(110)


@builtin_event(NodeType.ROLE_ASSIGNED_EVENT)
class RoleAssignedEvent(RoleEvent):
    """A Event regarding a Role."""

    pass


@builtin_event(NodeType.ROLE_UNASSIGNED_EVENT)
class RoleUnassignedEvent(RoleEvent):
    """A Event regarding a Role."""

    pass


@builtin_entity(
    NodeType.ROLE,
    event_types=(NodeType.ROLE_EVENT,),
    traits=(TraitType.OWNABLE,),
)
class Role(Entity):
    """A Role for Actors to take."""

    type: RoleType = builtin_property(100, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
