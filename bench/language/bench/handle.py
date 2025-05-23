from typing import TYPE_CHECKING

from bench.language.core import IsGlobal, IsInBench, IsSlug, Node, NodeType, node_
from bench.pb2 import HandleData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.HANDLE)
class Handle(IsGlobal, IsSlug, IsInBench, Node[HandleData]):
    """A Bench @handle."""

    pass
