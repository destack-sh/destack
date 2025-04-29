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
    ActionPage,
    AgentPage,
    BenchAgent,
    BenchAgentPage,
    CommonKit,
    ComputerDesktopKit,
    ComputerPage,
    ComputerTerminalKit,
    IComputerDesktop,
    IComputerTerminal,
    IInternet,
    InternetKit,
    InternetPage,
    ThreadPage,
    UbuntuComputerTemplate,
    UbuntuHeadlessComputerTemplate,
)
from .core import assign_builtin_ids, get_stable_builtin_path, sync_node

BenchPackage = Package(
    type=PackageType.OPEN,
    id=BENCH_BENCH_PACKAGE_ID,
    slug=BENCH_BENCH_PACKAGE_SLUG,
    _is_new=True,
)
BenchPackage.extend(ActionPage, ComputerPage, ThreadPage, AgentPage, InternetPage)

# finalize
supergraph = NodeSuperGraph(
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
    "ActionPage",
    "BenchAgent",
    "BenchAgentPage",
    "BenchPackage",
    "CommonKit",
    "ComputerDesktopKit",
    "ComputerPage",
    "ComputerTerminalKit",
    "IComputerDesktop",
    "IComputerTerminal",
    "IInternet",
    "InternetKit",
    "UbuntuComputerTemplate",
    "UbuntuHeadlessComputerTemplate",
    "assign_builtin_ids",
    "get_stable_builtin_path",
    "sync_node",
]
