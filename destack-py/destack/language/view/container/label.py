from destack.language.core import Node, NodeType, builtin_node
from destack.proto import LabelViewProto

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.LABEL_VIEW)
class LabelView(
    ContainerView,
    Node[LabelViewProto],
):
    """A label container View for form-like input views."""
