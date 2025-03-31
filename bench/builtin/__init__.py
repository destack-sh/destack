from bench.language import (
    BENCH_BENCH_ID,
    BENCH_BUILTIN_PACKAGE_ID,
    BENCH_BUILTIN_PACKAGE_SLUG,
    IsModal,
    NodeMode,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    Package,
    PackageType,
)

from .core import assign_builtin_ids, get_stable_builtin_path, sync_node
from .resource import BrowserPage, ComputerKit, ComputerPage, UbuntuComputerTemplate
from .source import ActionKit, ActionPage, BenchAgent, BenchFlow, FlowPage

BuiltinPackage = Package(
    type=PackageType.OPEN,
    id=BENCH_BUILTIN_PACKAGE_ID,
    slug=BENCH_BUILTIN_PACKAGE_SLUG,
    _is_new=True,
)
BuiltinPackage.extend(ActionPage, BrowserPage, FlowPage, ComputerPage)

# finalize
supergraph = NodeSuperGraph(
    name="Builtin", root_ptr=NodeReference(node_type=NodeType.BENCH, id=BENCH_BENCH_ID)
)
BuiltinPackage._graph.supergraph = supergraph
supergraph.add_graph(BuiltinPackage._graph)
for node in BuiltinPackage._graph.nodes:
    node._supergraph = supergraph
    if isinstance(node, IsModal) and node.mode == NodeMode.MAIN:
        node.mode = NodeMode.BUILTIN
assign_builtin_ids(BuiltinPackage._graph, ignore=(BuiltinPackage,))

__all__ = [
    "ActionKit",
    "ActionPage",
    "BenchAgent",
    "BenchFlow",
    "BrowserPage",
    "BuiltinPackage",
    "ComputerKit",
    "ComputerPage",
    "FlowPage",
    "UbuntuComputerTemplate",
    "assign_builtin_ids",
    "get_stable_builtin_path",
    "sync_node",
]
