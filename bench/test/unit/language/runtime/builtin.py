from bench.builtin import make_builtin_package, sync_node
from bench.language import Session
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime
from bench.test.utils import assert_graph_equals


def test_make_builtin_package(session: Session) -> None:
    """Make a Builtin package."""
    builtin_package = make_builtin_package()
    pass


@simulated_runtime()
async def test_sync_builtin_package(simulation: Simulation, runtime: RuntimeLambdaWorkload) -> None:
    """Sync the Builtins page."""
    builtin_package = make_builtin_package()
    empty_package = runtime.main_package

    # sync into empty package
    sync_node(parent=runtime.main_package, old_root=empty_package, new_root=builtin_package)
    assert runtime.session.tx.has_edits
    await runtime.commit()

    # should be equal
    assert_graph_equals(empty_package._graph, builtin_package._graph)

    # sync again (should have no further edits)
    sync_node(parent=runtime.main_package, old_root=empty_package, new_root=builtin_package)
    assert not runtime.session.tx.has_edits
    await runtime.commit()
