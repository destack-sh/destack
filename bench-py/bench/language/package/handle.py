from typing import TYPE_CHECKING

from bench.language.core import HasSlug, IsGlobal, IsInBench, Node, NodeType, node_
from bench.pb2 import HandleData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.HANDLE)
class Handle(
    IsGlobal,
    HasSlug,
    IsInBench,
    Node[HandleData],
):
    """A Bench @handle."""

    pass
