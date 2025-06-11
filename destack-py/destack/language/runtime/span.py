from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Analytic,
    IsFrozen,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    node_,
    property_parent_,
)
from destack.pb2 import SpanData

if TYPE_CHECKING:
    from destack.language import Run


# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SPAN)
class Span(
    Spatial,
    Analytic,
    IsTaggable,
    IsFrozen,
    Node[SpanData],
):
    """
    A Span is a trace inside a Run.
    """

    # meta
    parent: Union["Run", None] = property_parent_(node_is_customizable=False)
