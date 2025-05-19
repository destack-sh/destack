from bench.language import (
    BENCH_BENCH_PACKAGE_ID,
    BENCH_BENCH_PACKAGE_SLUG,
    BENCH_ID,
    IsModal,
    NodeMode,
    NodeReference,
    NodeType,
    Package,
    PackageType,
    Supergraph,
)

from .bench import (
    AgentPage,
    BenchAgent,
    BenchAgentPage,
    IInternetService,
    InternetPage,
    InternetService,
    ThreadPage,
)
from .core import assign_builtin_ids, get_stable_builtin_path, sync_node

BenchPackage = Package(
    type=PackageType.OPEN,
    id=BENCH_BENCH_PACKAGE_ID,
    slug=BENCH_BENCH_PACKAGE_SLUG,
    _is_new=True,
)
BenchPackage.add_children(ThreadPage, AgentPage, InternetPage)

# finalize
supergraph = Supergraph(
    name="Builtin", root_ptr=NodeReference(node_type=NodeType.BENCH, id=BENCH_ID)
)
BenchPackage._graph.supergraph = supergraph
supergraph.add_graph(BenchPackage._graph)
for node in BenchPackage._graph.nodes:
    node._supergraph = supergraph
    if isinstance(node, IsModal) and node.mode == NodeMode.MAIN:
        node.mode = NodeMode.BUILTIN
assign_builtin_ids(BenchPackage._graph, ignore=(BenchPackage,))

__all__ = [
    "BenchAgent",
    "BenchAgentPage",
    "BenchPackage",
    "IInternetService",
    "InternetService",
    "assign_builtin_ids",
    "get_stable_builtin_path",
    "sync_node",
]
