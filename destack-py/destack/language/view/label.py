from destack.language.core import NodeType, builtin_node

from .layout import LayoutView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.LABEL_VIEW)
class LabelView(LayoutView):
    """A label container View for form-like input views."""

    pass
