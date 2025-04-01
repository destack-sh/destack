from bench.language import (
    BENCH_BENCH_PACKAGE_ID,
    BENCH_BENCH_PACKAGE_SLUG,
    BENCH_ID,
    IsModal,
    NodeMode,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    Package,
    PackageType,
)

from .bench import (
    ActionKit,
    ActionPage,
    BenchAgent,
    BenchFlow,
    ComputerKit,
    ComputerPage,
    FlowPage,
    UbuntuComputerTemplate,
)
from .core import assign_builtin_ids, get_stable_builtin_path, sync_node

BuiltinPackage = Package(
    type=PackageType.OPEN,
    id=BENCH_BENCH_PACKAGE_ID,
    slug=BENCH_BENCH_PACKAGE_SLUG,
    _is_new=True,
)
BuiltinPackage.extend(ActionPage, FlowPage, ComputerPage)

# finalize
supergraph = NodeSuperGraph(
    name="Builtin", root_ptr=NodeReference(node_type=NodeType.BENCH, id=BENCH_ID)
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
    "BuiltinPackage",
    "ComputerKit",
    "ComputerPage",
    "FlowPage",
    "UbuntuComputerTemplate",
    "assign_builtin_ids",
    "get_stable_builtin_path",
    "sync_node",
]

# TODO: reorganize builtins .. (mirroring language seems wrong)
