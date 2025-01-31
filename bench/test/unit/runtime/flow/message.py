from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_flow_from_message(runtime: RuntimeLambdaWorkload):
    pass  # nocheckin
