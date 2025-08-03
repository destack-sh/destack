from destack.core import NodeType, builtin_entity

from .layout import LayoutView

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.LABEL_VIEW,
)
class LabelView(LayoutView):
    """A label container View for form-like input views."""

    pass
