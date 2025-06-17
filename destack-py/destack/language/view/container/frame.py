from destack.language.core import Node, NodeType, builtin_node
from destack.proto import FrameViewProto

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FRAME_VIEW)
class FrameView(
    ContainerView,
    Node[FrameViewProto],
):
    """A frame container View."""
