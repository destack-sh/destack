from destack.core import NodeType, declare_entity

from .layout import LayoutView

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(
    NodeType.LABEL_VIEW,
)
class LabelView(LayoutView):
    """A label container View for form-like input views."""

    pass
