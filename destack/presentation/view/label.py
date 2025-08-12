from destack.core import NodeType, declare_entity

from .layout import LayoutView2D


@declare_entity(
    NodeType.LABEL_VIEW2D,
)
class LabelView2D(LayoutView2D):
    """A label container View for form-like input views."""

    pass
