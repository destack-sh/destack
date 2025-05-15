from bench.language.core import NodeType, node_
from bench.pb2 import FrameViewData

from .container import ContainerViewBase


@node_(NodeType.FRAME_VIEW)
class FrameView(ContainerViewBase[FrameViewData]):
    """A frame container View."""
