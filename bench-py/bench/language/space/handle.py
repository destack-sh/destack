from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    HasSlug,
    IsGlobal,
    IsInSpace,
    IsTracked,
    Node,
    NodeType,
    node_,
    property_parent_,
)
from bench.pb2 import HandleData

if TYPE_CHECKING:
    from bench.language import Space

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.HANDLE)
class Handle(
    HasSlug,
    IsGlobal,
    IsInSpace,
    IsTracked,
    Node[HandleData],
):
    """A Bench @handle."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
