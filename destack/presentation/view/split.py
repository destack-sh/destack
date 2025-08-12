from destack.core import NodeType, declare_entity

from .layout import LayoutView2D


@declare_entity(
    NodeType.SPLIT_VIEW2D,
)
class SplitView2D(LayoutView2D):
    """A 2D split container View."""

    pass
