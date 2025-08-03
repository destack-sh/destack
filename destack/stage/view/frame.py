from destack.core import NodeType, declare_entity

from .layout import LayoutView

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(
    NodeType.FRAME_VIEW,
)
class FrameView(LayoutView):
    """
    A frame View is a bare ContainerView.
    """

    pass
