from typing import TYPE_CHECKING

from destack.core import (
    Event,
    NodeType,
    declare_entity,
    declare_event,
)

from ..geometry import Entity2D, Entity3D

if TYPE_CHECKING:
    pass


@declare_event(
    NodeType.BODY_EVENT,
    is_abstract=True,
)
class BodyEvent(Event):
    """An Event regarding a Body."""


@declare_event(
    NodeType.BODY_SLEEP_EVENT,
)
class BodySleepEvent(BodyEvent):
    """An Event regarding a Body Sleep."""

    pass


@declare_event(
    NodeType.BODY_WAKE_EVENT,
)
class BodyWakeEvent(BodyEvent):
    """An Event regarding a Body Wake."""

    pass


@declare_entity(
    NodeType.BODY2D,
    event_types=(NodeType.BODY_EVENT,),
    is_abstract=True,
)
class Body2D(Entity2D):
    pass


@declare_entity(
    NodeType.BODY3D,
    event_types=(NodeType.BODY_EVENT,),
    is_abstract=True,
)
class Body3D(Entity3D):
    pass
