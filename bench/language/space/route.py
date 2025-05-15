from bench.language.core import IsInstantiable, IsModal, IsOwnable, NodeType, PageNode, node_
from bench.pb2 import RouteData

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ROUTE)
class Route(IsOwnable, IsInstantiable, IsModal, PageNode[RouteData]):
    pass
