from bench.language import (
    BENCH_BUILTIN_PACKAGE_ID,
    BENCH_BUILTIN_PACKAGE_SLUG,
    IsModal,
    NodeMode,
    Package,
    PackageType,
)

from .core import assign_builtin_ids
from .resource import BrowserPage, ComputerPage
from .source import ActionPage, FlowPage

BuiltinPackage = Package(
    type=PackageType.OPEN,
    id=BENCH_BUILTIN_PACKAGE_ID,
    slug=BENCH_BUILTIN_PACKAGE_SLUG,
    _is_new=True,
)
BuiltinPackage.extend(ActionPage, BrowserPage, FlowPage, ComputerPage)

# finalize
# supergraph = NodeSuperGraph(
#     name="Builtin", root_ptr=NodeReference(node_type=NodeType.BENCH, id=BENCH_BENCH_ID)
# )
# BuiltinPackage._graph.supergraph = supergraph
# supergraph.add_graph(BuiltinPackage._graph)
for node in BuiltinPackage._graph.nodes:
    # node._supergraph = supergraph
    if isinstance(node, IsModal) and node.mode == NodeMode.MAIN:
        node.mode = NodeMode.BUILTIN
assign_builtin_ids(BuiltinPackage._graph, ignore=(BuiltinPackage,))
