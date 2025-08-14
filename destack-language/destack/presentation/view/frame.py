from destack.core import NodeType, declare_entity

from .layout import LayoutView2D


@declare_entity(
    NodeType.FRAME_VIEW2D,
)
class FrameView2D(LayoutView2D):
    """
    A frame View is a bare ContainerView.
    """

    pass
