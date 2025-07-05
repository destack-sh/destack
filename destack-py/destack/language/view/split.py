from destack.language.core import NodeType, builtin_node

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SPLIT_VIEW)
class SplitView(ContainerView):
    """A split container View."""

    pass
