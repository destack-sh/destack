from bench.language import Block, BlockType
from bench.runtime import create_run_from_node
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import ClientLambdaWorkload, RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_client, simulated_runtime

# NOTE: 'scheduler' == entirety of Plugins and systems to create/continue Runs somehow
#  (on Triggers or when manually requested by creating Runs in a Client)


@simulated_runtime()
async def test_run_flow_from_message(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    pass  # nocheckin


@simulated_client(runtimes=True)
async def test_run(simulation: Simulation, client: ClientLambdaWorkload):
    """Create a Run and wait for it to be executed."""
    page = client.page()
    flow = Block.new(BlockType.FLOW, "Flow")
    page.blocks.append(flow)
    await client.commit()

    run = create_run_from_node(flow)
    # nocheckin: wait for Run? ensure RuntimeHandle waits for completion?
    #  (this feels related to Runtime.wait_for and also the load remote nodes thing)
