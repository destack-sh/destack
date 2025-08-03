from destack.core import NodeType, builtin_entity

from .layout import LayoutView

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.FRAME_VIEW,
)
class FrameView(LayoutView):
    """
    A frame View is a bare ContainerView.
    """

    pass
