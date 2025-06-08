from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsEnvironmental,
    IsFrozen,
    IsInPackage,
    Node,
    NodeType,
    node_,
    property_parent_,
)
from bench.pb2 import SpanData

if TYPE_CHECKING:
    from bench.language import Run


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
