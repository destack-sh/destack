from destack.language.core import Node, NodeType, builtin_node

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FRAME_VIEW)
class FrameView(
    ContainerView,
    Node,
):
    """A frame container View."""
