from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    Event,
    IndexIn,
    IsOwned,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import IsFollowable

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.FOLLOW,
    index=(IndexIn(columns=("parent_id", "owned_by_id"), is_unique=True),),
)
class Follow(
    IsOwned,
    Entity,
):
    """A Follow is a relationship between a Actor and an IsFollowable Node."""

    parent: Union["IsFollowable", None] = builtin_property_parent()


@builtin_node(NodeType.FOLLOW_EVENT, frozen=True)
class FollowEvent(Event["Follow"]):
    node: "Follow" = builtin_property(101)


@builtin_node(NodeType.FOLLOW_ADDED_EVENT, frozen=True)
class FollowAddedEvent(FollowEvent):
    pass


@builtin_node(NodeType.FOLLOW_REMOVED_EVENT, frozen=True)
class FollowRemovedEvent(FollowEvent):
    pass
