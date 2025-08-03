from typing import TYPE_CHECKING

from destack.language.core import (
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
    NodeType.REACTION,
    traits=(TraitType.OWNED,),
    constraints=(
        ConstraintDeclaration(
            id=1,
            type=ConstraintType.UNIQUE,
            properties=("parent", "owned_by", "content"),
        ),
    ),
)
class Reaction(Entity):
    """A Reaction is a relationship between a Actor and a Reaction Node."""

    content: str = builtin_property(101, is_repr=True)


@builtin_event(NodeType.REACTION_EVENT)
class ReactionEvent(Event):
    reaction: "Reaction" = builtin_property(101)
    content: str = builtin_property(102)


@builtin_event(NodeType.REACTION_ADDED_EVENT)
class ReactionAddedEvent(ReactionEvent):
    pass


@builtin_event(NodeType.REACTION_REMOVED_EVENT)
class ReactionRemovedEvent(ReactionEvent):
    pass
