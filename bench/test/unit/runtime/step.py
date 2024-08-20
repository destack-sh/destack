from bench.language.block import Block
from bench.language.const import BlockType
from bench.language.step import Step, StepType
from bench.test.unit.conftest import RuntimeHandle


async def test_run_flow_empty(local_runtime: RuntimeHandle):
    """Empty Code without any fields should fail."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    run = await local_runtime.run(Flow1)
    assert run.run and not run.run.runs  # no nested runs


async def test_run_flow_trivial_no_value(local_runtime: RuntimeHandle):
    """Trivial flow with Start->Complete, no value."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Complete.connect(Start)
    Flow1.steps.extend(Start, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    run = await local_runtime.run(Flow1)
    assert run.run and len(run.run.runs) == 2  # two steps
