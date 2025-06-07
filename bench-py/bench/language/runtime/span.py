from typing import TYPE_CHECKING, Union

from bench.language.core import (
    HasEnvironment,
    IsInPackage,
    Node,
    NodeType,
    SpanType,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import SpanData

if TYPE_CHECKING:
    from bench.language import Run


# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SPAN)
class Span(
    HasEnvironment,
    IsInPackage,
    Node[SpanData],
):
    """
    A Span is a sub-part of a Run that represents a small, isolated unit of work inside a Run.
    """

    # meta
    parent: Union["Run", None] = property_parent_()
    type: SpanType = property_(30)
    # content
    title: str | None = property_(60)
