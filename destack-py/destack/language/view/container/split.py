from destack.language.core import Node, NodeType, builtin_node

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SPLIT_VIEW)
class SplitView(
    ContainerView,
    Node,
):
    """A split container View."""
