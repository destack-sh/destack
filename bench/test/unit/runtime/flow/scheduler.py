from bench.language import Block, BlockType, RunStatus
from bench.runtime import create_run_from_node
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime

# NOTE: 'scheduler' == entirety of Plugins and systems to create/continue Runs somehow
#  (on Triggers or when manually requested by creating Runs in a Client)


@simulated_runtime(runtimes=True)
async def test_run(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run and wait for it to execute in another Runtime."""
    page = runtime.page()
    flow = Block.new(BlockType.FLOW, "Flow")
    page.blocks.append(flow)
    await runtime.commit()

    run = create_run_from_node(flow)
    await run.wait_until_terminated()
    assert run.status == RunStatus.COMPLETED


@simulated_runtime()
async def test_run_flow_from_message(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    raise NotImplementedError("nocheckin")
