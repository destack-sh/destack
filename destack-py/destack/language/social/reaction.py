from typing import TYPE_CHECKING

from destack.language.core import (
    ConstraintDeclaration,
    ConstraintType,
    Entity,
    Event,
    IsOwned,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.REACTION,
    constraints=(
        ConstraintDeclaration(
            id=1,
            type=ConstraintType.UNIQUE,
            properties=("parent", "owned_by", "content"),
        ),
    ),
)
class Reaction(IsOwned, Entity):
    """A Reaction is a relationship between a Actor and a Reaction Node."""

    content: str = builtin_property(101, is_repr=True)


@builtin_node(NodeType.REACTION_EVENT, frozen=True)
class ReactionEvent(Event["Reaction"]):
    node: "Reaction" = builtin_property(101)
    content: str = builtin_property(102)


@builtin_node(NodeType.REACTION_ADDED_EVENT, frozen=True)
class ReactionAddedEvent(ReactionEvent):
    pass


@builtin_node(NodeType.REACTION_REMOVED_EVENT, frozen=True)
class ReactionRemovedEvent(ReactionEvent):
    pass
