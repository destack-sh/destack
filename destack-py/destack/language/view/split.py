from destack.language.core import NodeType, builtin_node

from .layout import LayoutView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SPLIT_VIEW)
class SplitView(LayoutView):
    """A split container View."""

    pass
