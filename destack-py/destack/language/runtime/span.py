from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Event,
    Node,
    NodeType,
    builtin_node,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Run


# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SPAN)
class Span(
    Event,
    Node,
):
    """
    A Span is a trace inside a Run.
    """

    # meta
    parent: Union["Run", None] = property_parent_(node_is_customizable=False)
