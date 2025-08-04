from destack.core import NodeType, declare_entity

from .layout import LayoutView


@declare_entity(
    NodeType.SPLIT_VIEW,
)
class SplitView(LayoutView):
    """A split container View."""

    pass
