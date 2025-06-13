from destack.language.core import Node, NodeType, builtin_node
from destack.pb2 import FrameViewData

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FRAME_VIEW)
class FrameView(
    ContainerView,
    Node[FrameViewData],
):
    """A frame container View."""
