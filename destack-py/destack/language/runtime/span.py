from typing import TYPE_CHECKING, Union

from destack.language.core import (
    IsEnvironmental,
    IsFrozen,
    IsInPackage,
    Node,
    NodeType,
    node_,
    property_parent_,
)
from destack.pb2 import SpanData

if TYPE_CHECKING:
    from destack.language import Run


# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SPAN)
class Span(
    IsEnvironmental,
    IsFrozen,
    IsInPackage,
    Node[SpanData],
):
    """
    A Span is a trace inside a Run.
    """

    # meta
    parent: Union["Run", None] = property_parent_(node_is_customizable=False)
