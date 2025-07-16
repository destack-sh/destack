from typing import TYPE_CHECKING, Union

from destack.language.core import (
    ConstraintDeclaration,
    ConstraintType,
    Entity,
    Event,
    IsOwned,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import IsStarable

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.STAR,
    constraints=(
        ConstraintDeclaration(
            id=1,
            type=ConstraintType.UNIQUE,
            properties=("parent", "owned_by"),
        ),
    ),
)
class Star(
    IsOwned,
    Entity,
):
    """A Star is a relationship between a Actor and a Starred Node."""

    parent: Union["IsStarable", None] = builtin_property_parent()


@builtin_node(NodeType.STAR_EVENT, frozen=True)
class StarEvent(Event["Star"]):
    node: "Star" = builtin_property(101)


@builtin_node(NodeType.STAR_ADDED_EVENT, frozen=True)
class StarAddedEvent(StarEvent):
    pass


@builtin_node(NodeType.STAR_REMOVED_EVENT, frozen=True)
class StarRemovedEvent(StarEvent):
    pass
