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


@builtin_node(NodeType.STAR_EVENT, frozen=True)
class StarEvent(Event["Star"]):
    node: "Star" = builtin_property(101)


@builtin_node(NodeType.STAR_ADDED_EVENT, frozen=True)
class StarAddedEvent(StarEvent):
    pass


@builtin_node(NodeType.STAR_REMOVED_EVENT, frozen=True)
class StarRemovedEvent(StarEvent):
    pass
