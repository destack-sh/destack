from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    HasSlug,
    IsGlobal,
    IsInBench,
    IsTracked,
    Node,
    NodeType,
    node_,
    property_parent_,
)
from bench.pb2 import HandleData

if TYPE_CHECKING:
    from bench.language import Bench

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.HANDLE)
class Handle(
    HasSlug,
    IsGlobal,
    IsInBench,
    IsTracked,
    Node[HandleData],
):
    """A Bench @handle."""

    parent: Optional["Bench"] = property_parent_()
