from typing import TYPE_CHECKING

from destack.language.core import (
    ConstraintDeclaration,
    ConstraintType,
    Entity,
    Event,
    NodeType,
    TraitType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
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


@builtin_node(NodeType.FOLLOW_EVENT, frozen=True)
class FollowEvent(Event["Follow"]):
    node: "Follow" = builtin_property(101)


@builtin_node(NodeType.FOLLOW_ADDED_EVENT, frozen=True)
class FollowAddedEvent(FollowEvent):
    pass


@builtin_node(NodeType.FOLLOW_REMOVED_EVENT, frozen=True)
class FollowRemovedEvent(FollowEvent):
    pass
