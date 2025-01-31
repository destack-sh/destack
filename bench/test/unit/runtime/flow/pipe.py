from bench.language import (
    Action,
    ActionType,
    Block,
    BlockType,
    PipeType,
)
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_flow_pipe_directly(runtime: RuntimeLambdaWorkload):
    """Run a Pipe directly."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Pipe = Start.connect(PipeType.CALL, Complete)
    runtime.page().blocks.append(Flow1)

    _ = await runtime.run_in_runtime(Pipe)
