from bench.language import sync_node
from bench.language.builtin import make_builtins
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_sync_builtins(simulation: Simulation, runtime: RuntimeLambdaWorkload) -> None:
    """Sync the Builtins page."""
    Builtins = make_builtins(runtime.session)
    sync_node(
        parent=runtime.main_package,
        old_root=runtime.main_package.blocks.get("Builtins"),
        new_root=Builtins,
    )
    assert runtime.session.tx.has_edits
    await runtime.commit()

    sync_node(
        parent=runtime.main_package,
        old_root=runtime.main_package.blocks.get("Builtins"),
        new_root=Builtins,
    )
    assert not runtime.session.tx.has_edits
    await runtime.commit()
