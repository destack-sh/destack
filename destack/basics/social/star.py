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


@declare_event(NodeType.STAR_EVENT)
class StarEvent(Event):
    star: "Star" = declare_property(
        101,
        reference_type=ReferenceType.LOCATION,
    )


@declare_event(NodeType.STAR_ADDED_EVENT)
class StarAddedEvent(StarEvent):
    pass


@declare_event(NodeType.STAR_REMOVED_EVENT)
class StarRemovedEvent(StarEvent):
    pass
