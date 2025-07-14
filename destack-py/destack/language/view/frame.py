from destack.language.core import NodeType, builtin_node

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FRAME_VIEW)
class FrameView(ContainerView):
    """
    A frame View is a bare ContainerView.
    """

    pass
