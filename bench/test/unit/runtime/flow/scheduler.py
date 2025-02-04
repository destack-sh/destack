from bench.language import Action, ActionType, Block, BlockType, RunStatus
from bench.language.source.trigger import Trigger
from bench.runtime import create_run_from_node
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime

# NOTE :Test: reorganize 'runtime' simulation tests (and maybe reorganize all tests?)


@simulated_runtime(runtimes=True)
async def test_run(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run and wait for it to execute in another Runtime."""
    Page1 = runtime.page()
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Page1.blocks.append(Flow1)
    await runtime.commit()

    run = create_run_from_node(Flow1)
    await run.wait_until_terminated()
    assert run.status == RunStatus.COMPLETED


@simulated_runtime()
async def test_run_flow_from_message(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run from a Message in a Flow."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Receive1 = Action.new(ActionType.RECEIVE, "Receive1", triggers=[Trigger.message(...)])
    raise NotImplementedError("nocheckin")
