from typing import TYPE_CHECKING

from destack.core import (
    ConstraintDeclaration,
    ConstraintType,
    Entity,
    Event,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.FOLLOW,
    traits=(TraitType.OWNED,),
    constraints=(
        ConstraintDeclaration(
            id=1,
            type=ConstraintType.UNIQUE,
            properties=("parent", "owned_by"),
        ),
    ),
)
class Follow(Entity):
    """A Follow is a relationship between a Actor and an IsFollowable Node."""

    pass


@builtin_event(NodeType.FOLLOW_EVENT)
class FollowEvent(Event):
    follow: "Follow" = builtin_property(101)


@builtin_event(NodeType.FOLLOW_ADDED_EVENT)
class FollowAddedEvent(FollowEvent):
    pass


@builtin_event(NodeType.FOLLOW_REMOVED_EVENT)
class FollowRemovedEvent(FollowEvent):
    pass
