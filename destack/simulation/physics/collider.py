from typing import TYPE_CHECKING

from destack.core import (
    Event,
    NodeType,
    ReferenceType,
    declare_entity,
    declare_event,
    declare_property,
)

from ..geometry import Entity2D, Entity3D

if TYPE_CHECKING:
    pass


@declare_event(
    NodeType.COLLIDER_EVENT,
    is_abstract=True,
)
class ColliderEvent(Event):
    """An Event regarding a Collider."""

    pass


@declare_event(
    NodeType.COLLIDER_CONTACT_EVENT,
)
class ColliderContactEvent(ColliderEvent):
    """An Event regarding a Collider Contact."""

    collider_a: "Collider2D" = declare_property(
        110, reference_type=ReferenceType.LOCATION, tag=None
    )
    collider_b: "Collider2D" = declare_property(
        111, reference_type=ReferenceType.LOCATION, tag=None
    )


@declare_entity(
    NodeType.COLLIDER2D,
)
class Collider2D(Entity2D):
    pass


@declare_entity(
    NodeType.COLLIDER3D,
)
class Collider3D(Entity3D):
    pass
