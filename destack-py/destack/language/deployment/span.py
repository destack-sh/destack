from datetime import datetime, timedelta
from typing import TYPE_CHECKING

from destack.language.core import (
    Event,
    NodeType,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.SPAN_EVENT)
class SpanEvent(Event):
    """
    A Span is a trace inside a Run.
    """

    name: str = builtin_property(101)
    start_time: datetime = builtin_property(102)
    end_time: datetime = builtin_property(103)
    duration: timedelta = builtin_property(104)

    parent_span: "SpanEvent" = builtin_property(110)
