from destack.language.core import Node, NodeType, node_
from destack.pb2 import LabelViewData

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.LABEL_VIEW)
class LabelView(
    ContainerView,
    Node[LabelViewData],
):
    """A label container View for form-like input views."""
