from destack.language.core import NodeType, builtin_node

from .layout import LayoutView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.FRAME_VIEW,
)
class FrameView(LayoutView):
    """
    A frame View is a bare ContainerView.
    """

    pass
