from bench.language import Session
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime
from bench.test.utils import assert_graph_equals


def test_make_builtin_package(session: Session) -> None:
    """Make a Builtin package."""
    from bench.builtin import BuiltinPackage

    _ = BuiltinPackage


@simulated_runtime()
async def test_sync_builtin_package(simulation: Simulation, runtime: RuntimeLambdaWorkload) -> None:
    """Sync the Builtins page."""
    from bench.builtin import BuiltinPackage, sync_node

    empty_package = runtime.main_package

    # sync into empty package
    sync_node(parent=runtime.main_package, old_root=empty_package, new_root=BuiltinPackage)
    assert runtime.session.tx.has_edits
    await runtime.commit()

    # should be equal
    assert_graph_equals(empty_package._graph, BuiltinPackage._graph)

    # sync again (should have no further edits)
    sync_node(parent=runtime.main_package, old_root=empty_package, new_root=BuiltinPackage)
    assert not runtime.session.tx.has_edits
    await runtime.commit()
