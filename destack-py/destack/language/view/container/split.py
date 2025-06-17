from destack.language.core import Node, NodeType, builtin_node
from destack.proto import SplitViewProto

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SPLIT_VIEW)
class SplitView(
    ContainerView,
    Node[SplitViewProto],
):
    """A split container View."""
