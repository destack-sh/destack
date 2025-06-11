from typing import TYPE_CHECKING, Union

from destack.language.core import (
    IsFrozen,
    IsParticle,
    IsSpatial,
    IsTaggable,
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
    IsTaggable,
    IsFrozen,
    IsParticle,
    IsSpatial,
    Node[SpanData],
):
    """
    A Span is a trace inside a Run.
    """

    # meta
    parent: Union["Run", None] = property_parent_(node_is_customizable=False)
