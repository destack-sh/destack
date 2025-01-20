from bench.language import (
    Action,
    ActionType,
    Block,
    BlockType,
    PipeType,
)
from bench.test.unit.conftest import RuntimeHandle


async def test_run_flow_pipe_directly(hosted_runtime: RuntimeHandle):
    """Run a Pipe directly."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Pipe = Start.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)

    _ = await hosted_runtime.run(Pipe)
