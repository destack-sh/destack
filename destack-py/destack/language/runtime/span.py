from typing import TYPE_CHECKING

from destack.language.core import (
    Event,
    NodeType,
    builtin_node,
    property_,
)

if TYPE_CHECKING:
    from destack.language import Run


# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SPAN_EVENT)
class SpanEvent(Event):
    """
    A Span is a trace inside a Run.
    """

    # meta
    run: "Run" = property_(40)
