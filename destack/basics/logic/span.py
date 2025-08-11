from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from destack.core import (
    Event,
    NodeType,
    ReferenceType,
    Value,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Action


@declare_event(NodeType.SPAN_EVENT)
class SpanEvent(Event):
    """
    A Span is a trace inside a Run.
    """

    custom_values: dict[str, "Value"] | None = declare_property(
        45,
        description="The custom Values of this Entity, keyed by custom Property name.",
    )

    name: str = declare_property(
        101,
        is_interned=True,
    )
    start_time: datetime = declare_property(102)
    end_time: datetime = declare_property(103)
    duration: timedelta = declare_property(104)

    parent_span: Optional["SpanEvent"] = declare_property(
        110,
        reference_type=ReferenceType.LOCATION,
    )
    action: Optional["Action"] = declare_property(
        111,
        reference_type=ReferenceType.LOCATION,
    )
