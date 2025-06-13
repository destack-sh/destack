from destack.language.core import Node, NodeType, builtin_node
from destack.pb2 import LabelViewData

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.LABEL_VIEW)
class LabelView(
    ContainerView,
    Node[LabelViewData],
):
    """A label container View for form-like input views."""
