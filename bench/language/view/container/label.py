from bench.language.core import IsArchivable, IsDeletable, Node, NodeType, node_
from bench.pb2 import LabelViewData

from .container import IsContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.LABEL_VIEW)
class LabelView(
    IsContainerView,
    IsDeletable,
    IsArchivable,
    Node[LabelViewData],
):
    """A label container View for form-like input views."""
