from destack.language.core import NodeType, builtin_entity

from .layout import LayoutView

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.SPLIT_VIEW,
)
class SplitView(LayoutView):
    """A split container View."""

    pass
