from destack.language.core import IsArchivable, IsDeletable, Node, NodeType, node_
from destack.pb2 import FrameViewData

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.FRAME_VIEW)
class FrameView(
    ContainerView,
    IsDeletable,
    IsArchivable,
    Node[FrameViewData],
):
    """A frame container View."""
