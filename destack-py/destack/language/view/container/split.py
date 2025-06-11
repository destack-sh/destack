from destack.language.core import Node, NodeType, node_
from destack.pb2 import SplitViewData

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SPLIT_VIEW)
class SplitView(
    ContainerView,
    Node[SplitViewData],
):
    """A split container View."""
