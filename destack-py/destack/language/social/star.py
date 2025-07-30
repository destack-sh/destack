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
    NodeType.STAR,
    traits=(TraitType.OWNED,),
    constraints=(
        ConstraintDeclaration(
            id=1,
            type=ConstraintType.UNIQUE,
            properties=("parent", "owned_by"),
        ),
    ),
)
class Star(Entity):
    """A Star is a relationship between a Actor and a Starred Node."""


@builtin_event(NodeType.STAR_EVENT)
class StarEvent(Event["Star"]):
    node: "Star" = builtin_property(101)


@builtin_event(NodeType.STAR_ADDED_EVENT)
class StarAddedEvent(StarEvent):
    pass


@builtin_event(NodeType.STAR_REMOVED_EVENT)
class StarRemovedEvent(StarEvent):
    pass
