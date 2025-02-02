from bench.language import ScalerStrategy, ScalerType
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_create_scaler(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Creates a scaler."""
    _ = runtime.bench.scalers.create(
        type=ScalerType.MACHINE,
        strategy=ScalerStrategy.AUTO,
        name="Machine Scaler",
        min_count=1,
        target_count=1,
        max_count=4,
    )
    await runtime.commit()
