from destack.language.core import Node, NodeType, builtin_node

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.LABEL_VIEW)
class LabelView(
    ContainerView,
    Node,
):
    """A label container View for form-like input views."""
