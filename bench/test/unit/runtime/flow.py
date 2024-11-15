import asyncio

from bench.language import ActionMode, BlockType, RunStatus
from bench.language.block import Block
from bench.language.code import code
from bench.language.field import Field
from bench.language.flow import PipeType, Step, StepType
from bench.language.run import RunErrorType, RunOptions
from bench.runtime.runner import run_from_node
from bench.test.unit.conftest import RuntimeHandle


async def test_run_step_directly(local_runtime: RuntimeHandle):
    """Run a Steps directly. Should work."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code = Step.new(StepType.ACTION, "Code", mode=ActionMode.STRICT, code=code("pass"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    _ = await local_runtime.run(Start)
    _ = await local_runtime.run(Code)
    _ = await local_runtime.run(Complete)


async def test_run_pipe_directly(local_runtime: RuntimeHandle):
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Complete)
    Pipe = Start.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)

    _ = await local_runtime.run(Pipe)


async def test_run_flow_empty(local_runtime: RuntimeHandle):
    """Empty Code without any fields should fail."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and not runner.run.runs  # no nested runs


async def test_run_flow_spurious(local_runtime: RuntimeHandle):
    """Flow with Steps that go nowhere."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Flow1.steps.append(Step.new(StepType.START, "Start"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Code1 = Step.new(StepType.ACTION, "Code1", mode=ActionMode.STRICT, code=code("pass"))
    # don't actually connect the steps
    Flow1.steps.extend(Start, Complete, Code1)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 1  # just Start


async def test_run_flow_trivial_no_value(local_runtime: RuntimeHandle):
    """Trivial flow with Start->Complete, no value."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Complete)
    Start.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 2  # two steps


async def test_run_flow_pipe_from_nowhere(local_runtime: RuntimeHandle):
    """Run a flow with a pipe from nowhere. Should not be run and just be ignored."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Complete)
    Nowhere = Step.new(StepType.START, "Nowhere")  # not added to flow/graph
    Nowhere.connect(PipeType.PASS, Complete, parent=Flow1)
    Start.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 2


async def test_run_flow_pipe_to_nowhere(local_runtime: RuntimeHandle):
    """Run a flow with a pipe to nowhere. Should not be run and just be ignored."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Complete)
    Nowhere = Step.new(StepType.START, "Nowhere")  # not added to flow/graph
    Start.connect(PipeType.PASS, Nowhere, parent=Flow1)
    Start.connect(PipeType.PASS, Complete)
    Nowhere.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 2


async def test_run_flow_force_invalid_output(local_runtime: RuntimeHandle):
    """Complete the flow with invalid output (should fail)."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.output("Output1", str, is_required=True),)
    )
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Complete)
    Start.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE


async def test_run_flow_force_invalid_input(local_runtime: RuntimeHandle):
    """Run a step with a trigger port that forces a Run of a Step with invalid inputs (should fail)."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.output("Output1", str, is_required=True),)
    )
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.ACTION, "Code1", fields=(Field.input("Input1", str, is_required=True),)
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.connect(PipeType.PASS, Code1)
    Code1.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE


async def test_run_flow_code(local_runtime: RuntimeHandle):
    """Run a code step with values."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.ACTION,
        "Code1",
        code=code("return 2 * Input1"),
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.connect(PipeType.PASS, Code1)
    Code1.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, inputs={"Input1": 2})
    assert runner.run and len(runner.run.runs) == 3  # three steps
    assert runner.outputs and runner.outputs.Output1 == 4


async def test_run_flow_error(local_runtime: RuntimeHandle):
    """Run a code step with an erro. Flow should abort and fail."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(StepType.ACTION, "Code1", code=code("raise ValueError"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.connect(PipeType.PASS, Code1)
    Code1.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.run and len(runner.run.runs) == 2


async def test_run_flow_error_with_error_suppressed(local_runtime: RuntimeHandle):
    """Run a code step with an error with failure suppressed. Flow should complete."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.ACTION,
        "Code1",
        code=code("raise ValueError('error')"),
        run_options=RunOptions(suppress_fail=True),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.connect(PipeType.PASS, Code1)
    Code1.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.status == RunStatus.COMPLETED
    assert runner.run and len(runner.run.runs) == 3


async def test_run_flow_race(local_runtime: RuntimeHandle):
    """Run multiple steps in parallel, losers should be aborted on completion of winner."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Race1 = Step.new(StepType.ACTION, "Race1", code=code("await asyncio.sleep(1)"))
    Race2 = Step.new(StepType.ACTION, "Race2", code=code("await asyncio.sleep(2)"))
    Race3 = Step.new(StepType.ACTION, "Race3", code=code("await asyncio.sleep(3)"))
    Flow1.steps.extend(Start, Race1, Race2, Race3, Complete)
    Start.connect(PipeType.PASS, Race1)
    Start.connect(PipeType.PASS, Race2)
    Start.connect(PipeType.PASS, Race3)
    Race1.connect(PipeType.PASS, Complete)
    Race2.connect(PipeType.PASS, Complete)
    Race3.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run
    aborted_runs = [r for r in runner.run.runs if r.status == RunStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers should be aborted
    assert len(runner.run.runs) == 5  # all steps should run exactly once
    # slowers steps should be aborted
    assert runner.runs[2].node == Race2 and runner.runs[2].status == RunStatus.ABORTED
    assert runner.runs[3].node == Race3 and runner.runs[3].status == RunStatus.ABORTED


async def test_run_flow_infinite_loop(local_runtime: RuntimeHandle):
    """Runs an infinite loop that's not infinite because it also completes immediately."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Loop = Step.new(StepType.ACTION, "Loop")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Loop, Complete)
    Start.connect(PipeType.PASS, Loop)
    Loop.connect(PipeType.PASS, Loop)  # infinite!
    Loop.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 4  # two steps


async def test_run_flow_infinite_loop_with_extra_hop(local_runtime: RuntimeHandle):
    """Runs an infinite loop that's not infinite because it also completes immediately."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(StepType.ACTION, "Code1")
    Code2 = Step.new(StepType.ACTION, "Code2")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Code2, Complete)
    Start.connect(PipeType.PASS, Code1)
    Code1.connect(PipeType.PASS, Code1)  # infinite!
    Code1.connect(PipeType.PASS, Code2)
    Code2.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    _ = await local_runtime.run(Flow1)


async def test_run_flow_abort(local_runtime: RuntimeHandle):
    """Run a long async flow script and abort it. All pending steps should be aborted."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.ACTION,
        "Code1",
        mode=ActionMode.STRICT,
        code=code("await asyncio.sleep(5)"),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow.steps.extend(Start, Code1, Complete)
    Start.connect(PipeType.PASS, Code1)
    Code1.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    run = run_from_node(Flow)
    run_task = asyncio.create_task(local_runtime.run(run, return_error=True))
    # kill after 0.5s
    await asyncio.sleep(0.5)
    await local_runtime.runtime.abort_run(run)
    runner = await run_task
    # flow should be aborted
    assert runner.status == RunStatus.ABORTED
    assert runner.run and runner.run.duration and runner.run.duration.total_seconds() < 1
    # code step should also be aborted
    assert runner.runs[1].node == Code1 and runner.runs[1].status == RunStatus.ABORTED


async def test_run_flow_yield(local_runtime: RuntimeHandle):
    """Run a Flow with a Yield step, then resume from the Yield."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Yield = Step.new(StepType.YIELD, "Yield")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow.steps.extend(Start, Yield, Complete)
    Start.connect(PipeType.PASS, Yield)
    Yield.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow)
    # "nocheckin: pause/resume flows"
