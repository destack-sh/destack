from typing import TYPE_CHECKING, Optional

from destack.core import (
    Duration,
    Event,
    NodeType,
    ReferenceType,
    Timestamp,
    Value,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Action


@declare_event(NodeType.SPAN_EVENT)
class SpanEvent(Event):
    """
    A Span traces a Run.
    """

    custom_values: dict[str, "Value"] | None = declare_property(
        45,
        description="The custom Values of this Entity, keyed by custom Property name.",
        tag=None,
    )

    name: str = declare_property(
        101,
        is_interned=True,
        tag=None,
    )
    start_time: Timestamp = declare_property(102, tag=None)
    end_time: Timestamp = declare_property(103, tag=None)
    duration: Duration = declare_property(104, tag=None)

    parent_span: Optional["SpanEvent"] = declare_property(
        110,
        reference_type=ReferenceType.LOCATION,
        tag=None,
    )
    action: Optional["Action"] = declare_property(
        111,
        reference_type=ReferenceType.LOCATION,
        tag=None,
    )
