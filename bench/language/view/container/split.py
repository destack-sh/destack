from bench.language.core import NodeType, node_
from bench.pb2 import SplitViewData

from .container import ContainerViewBase


@node_(NodeType.SPLIT_VIEW)
class SplitView(ContainerViewBase[SplitViewData]):
    """A split container View."""
