from bench.language.core import NodeType, node_
from bench.pb2 import LabelViewData

from .container import ContainerViewBase


@node_(NodeType.LABEL_VIEW)
class LabelView(ContainerViewBase[LabelViewData]):
    """A label container View for form-like input views."""
