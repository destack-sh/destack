from destack.language.core import IsArchivable, IsDeletable, Node, NodeType, node_
from destack.pb2 import FrameViewData

from .container import IsContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.FRAME_VIEW)
class FrameView(
    IsContainerView,
    IsDeletable,
    IsArchivable,
    Node[FrameViewData],
):
    """A frame container View."""
