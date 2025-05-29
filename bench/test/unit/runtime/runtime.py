from bench.language import (
    Node,
    Package,
    Session,
)
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime
from bench.test.utils import assert_graph_equals
from bench.utils.func import reload_module


def test_make_builtin_package(session: Session) -> None:
    """Make a Builtin package. Make it again and check they're equal."""
    from bench import builtin

    prev_graph = builtin.BenchPackage._graph.copy()

    reload_module(builtin)

    from bench import builtin

    assert prev_graph is not builtin.BenchPackage._graph
    assert_graph_equals(prev_graph, builtin.BenchPackage._graph)

    # print for reference :Builtins
    node_by_path: dict[str, Node] = {}
    for node in builtin.BenchPackage._graph.nodes:
        path = builtin.get_stable_builtin_path(node)
        node_by_path[path] = node

    paths = sorted(node_by_path.keys())
    for path in paths:
        node = node_by_path[path]
        print(f"{node.id} - {path} - {node.metatype.name}")  # noqa: T201


@simulated_runtime(system=True)
async def test_builtin_package(simulation: Simulation, runtime: RuntimeLambdaWorkload):  # noqa: RUF029
    """Test the Builtin package."""
    from bench.builtin import BenchPackage as BuiltinPackageRaw
    from bench.language import BENCH_BENCH_PACKAGE_PTR

    # builtin graph in memory and builtin graph loaded from runtime/bench should be equal
    BuiltinPackageLoaded = runtime.main_package._supergraph.get_or_error(BENCH_BENCH_PACKAGE_PTR)
    assert isinstance(BuiltinPackageLoaded, Package)
    BuiltinPackageLoadedGraph = BuiltinPackageLoaded._graph.copy()
    loaded_bench_bench = BuiltinPackageLoaded.parent
    assert loaded_bench_bench is not None
    if (main_database := loaded_bench_bench.database) is not None:
        BuiltinPackageLoadedGraph.remove(main_database)
    BuiltinPackageLoadedGraph.remove(loaded_bench_bench, recursive=False)
    assert_graph_equals(BuiltinPackageRaw._graph, BuiltinPackageLoadedGraph)
