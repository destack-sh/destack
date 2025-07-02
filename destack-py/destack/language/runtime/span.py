from typing import TYPE_CHECKING

from destack.language.core import (
    Event,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Run


# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SPAN_EVENT)
class SpanEvent(Event["Run"]):
    """
    A Span is a trace inside a Run.
    """

    node: "Run" = builtin_property(101)
