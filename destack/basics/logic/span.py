from datetime import datetime, timedelta
from typing import TYPE_CHECKING

from destack.core import (
    Event,
    NodeType,
    Value,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    from destack import Action


# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.SPAN_EVENT)
class SpanEvent(Event):
    """
    A Span is a trace inside a Run.
    """

    custom_values: dict[str, "Value"] | None = builtin_property(
        45,
        description="The custom Values of this Entity, keyed by custom Property name.",
    )

    name: str = builtin_property(101)
    start_time: datetime = builtin_property(102)
    end_time: datetime = builtin_property(103)
    duration: timedelta = builtin_property(104)

    parent_span: "SpanEvent | None" = builtin_property(110)
    action: "Action | None" = builtin_property(111)
