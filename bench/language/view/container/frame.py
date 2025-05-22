from bench.language.core import Node, NodeType, node_
from bench.pb2 import FrameViewData

from .container import IsContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.FRAME_VIEW)
class FrameView(IsContainerView, Node[FrameViewData]):
    """A frame container View."""
