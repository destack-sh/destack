from typing import TYPE_CHECKING

from destack.core import (
    ConstraintDeclaration,
    ConstraintType,
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
    pass


@declare_entity(
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


@declare_event(NodeType.FOLLOW_EVENT)
class FollowEvent(Event):
    follow: "Follow" = declare_property(
        101,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )


@declare_event(NodeType.FOLLOW_ADDED_EVENT)
class FollowAddedEvent(FollowEvent):
    pass


@declare_event(NodeType.FOLLOW_REMOVED_EVENT)
class FollowRemovedEvent(FollowEvent):
    pass
