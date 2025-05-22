from bench.language.core import IsArchivable, IsDeletable, Node, NodeType, node_
from bench.pb2 import SplitViewData

from .container import IsContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SPLIT_VIEW)
class SplitView(
    IsContainerView,
    IsDeletable,
    IsArchivable,
    Node[SplitViewData],
):
    """A split container View."""
