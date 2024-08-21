import pytest

from bench.language.block import Block
from bench.language.code import code
from bench.language.const import BlockType, RunStatus
from bench.language.field import Field
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
    Start.then(Complete)
    Flow1.steps.extend(Start, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    run = await local_runtime.run(Flow1)
    assert run.run and len(run.run.runs) == 2  # two steps


async def test_run_flow_trivial_simple_value(local_runtime: RuntimeHandle):
    """Trivial flow with Start->Pass->Complete, single value."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.input("Input1", int), Field.output("Output1", int))
    )
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Start.then(Complete)
    Flow1.steps.extend(Start, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    run = await local_runtime.run(Flow1, inputs={"Input1": 2})
    assert run.run and len(run.run.runs) == 2  # two steps
    assert run.outputs and run.outputs.Output1 == 2


async def test_run_flow_race(local_runtime: RuntimeHandle):
    """Run multiple steps in parallel, losers should be aborted on completion of winner."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Race1 = Step.new(StepType.CODE, "Race1", code=code("await asyncio.sleep(0.1)"))
    Start.then(Race1).then(Complete)
    Race2 = Step.new(StepType.CODE, "Race2", code=code("await asyncio.sleep(0.2)"))
    Start.then(Race2).then(Complete)
    Race3 = Step.new(StepType.CODE, "Race3", code=code("await asyncio.sleep(0.3)"))
    Race3.then(Complete).then(Complete)
    Flow1.steps.extend(Start, Race1, Race2, Race3, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    run = await local_runtime.run(Flow1)
    assert run.run and len(run.run.runs) == 5  # all steps should run
    aborted_runs = [r for r in run.run.runs if r.status == RunStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers


async def test_run_flow_not_so_infinite_loop(local_runtime: RuntimeHandle):
    """Runs an infinite loop that's not infinite because it also completes immediately."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Loop = Step.new(StepType.PASS, "Loop")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Loop.then(Loop)
    Start.then(Loop).then(Complete)
    Flow1.steps.extend(Start, Loop, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    run = await local_runtime.run(Flow1)
    assert run.run and len(run.run.runs) == 2  # two steps


async def test_run_flow_code_block(local_runtime: RuntimeHandle):
    """Run a series of code block steps."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.input("Input1", int), Field.output("Output1", int))
    )
    CodeBlock1 = Block.new(
        BlockType.CODE,
        "Code1",
        code=code("return 2 * Input1"),
        fields=(Field.input("Input1", int), Field.output("Output1", int)),
    )
    CodeBlock2 = Block.new(
        BlockType.CODE,
        "Code2",
        code=code("return 3 * Input1"),
        fields=(Field.input("Input1", int),),
    )
    Code1 = Step.new(StepType.CODE, "Code1", node=CodeBlock1)
    Code2 = Step.new(StepType.CODE, "Code2", node=CodeBlock2)
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Start.then(Code1).then(Code2).then(Complete)
    Flow1.steps.extend(Start, Code1, Code2, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    run = await local_runtime.run(Flow1, inputs={"Input1": 2})
    assert run.outputs and run.outputs.Output1 == 12


@pytest.mark.skip()
async def test_run_flow_nested_flow(local_runtime: RuntimeHandle):
    """Run a FlowBlock step within a Flow (nested)."""
    ...
