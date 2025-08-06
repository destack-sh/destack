from typing import TYPE_CHECKING

from destack.core import (
    ConstraintDeclaration,
    ConstraintType,
    Entity,
    Event,
    NodeType,
    TraitType,
    declare_entity,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_entity(
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

    content: str = declare_property(101, is_repr=True)


@declare_event(NodeType.REACTION_EVENT)
class ReactionEvent(Event):
    reaction: "Reaction" = declare_property(101)
    content: str = declare_property(102)


@declare_event(NodeType.REACTION_ADDED_EVENT)
class ReactionAddedEvent(ReactionEvent):
    pass


@declare_event(NodeType.REACTION_REMOVED_EVENT)
class ReactionRemovedEvent(ReactionEvent):
    pass
