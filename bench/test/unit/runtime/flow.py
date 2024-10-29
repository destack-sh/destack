import asyncio

from bench.language.block import Block
from bench.language.code import code
from bench.language.const import ActionMode, BlockType, RunStatus
from bench.language.field import Field
from bench.language.flow import Step, StepType
from bench.language.run import Run, RunErrorType, RunOptions
from bench.test.unit.conftest import RuntimeHandle


async def test_run_step_directly(local_runtime: RuntimeHandle):
    """Run a Steps directly."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code = Step.new(StepType.ACTION, "Code", mode=ActionMode.STATIC, code=code("pass"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    _ = await local_runtime.run(Start)
    _ = await local_runtime.run(Code)
    _ = await local_runtime.run(Complete)


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
    Code1 = Step.new(StepType.ACTION, "Code1", mode=ActionMode.STATIC, code=code("pass"))
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
    Start.then(Complete)
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
    Nowhere.then(Complete, parent=Flow1)
    Start.then(Complete)
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
    Start.then(Nowhere, parent=Flow1)
    Start.then(Complete)
    Nowhere.then(Complete)
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
    Start.then(Complete)
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
    Start.then(Code1).then(Code1).then(Complete)
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
    Start.then(Code1).then(Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, inputs={"Input1": 2})
    assert runner.run and len(runner.run.runs) == 3  # three steps
    assert runner.outputs and runner.outputs.Output1 == 4


async def test_run_flow_multi_port(local_runtime: RuntimeHandle):
    """Run a a step with multiple input ports, only some of which are required. Should run as soon as all required inputs are provided."""
    Flow = Block.new(BlockType.FLOW, "Flow", fields=(Field.input("Input1", int, is_required=True),))
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.ACTION,
        "Code3",
        code=code("pass"),
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.input("Input2", int, is_required=True),
            Field.input("Input3", int),
        ),
    )
    Flow.steps.extend(Start, Code1)
    # connect Start.Input1 to both Code1.Input1 and Code1.Input2, should fire
    Start.then(Code1, source_port=Flow.fields.Input1, target_port=Code1.fields.Input1)
    Start.then(Code1, source_port=Flow.fields.Input1, target_port=Code1.fields.Input2)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow, inputs={"Input1": 1})
    assert runner.run and len(runner.run.runs) == 2  # Start and Code1 (once)


async def test_run_flow_error(local_runtime: RuntimeHandle):
    """Run a code step with an erro. Flow should abort and fail."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(StepType.ACTION, "Code1", code=code("raise ValueError"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.then(Code1).then(Complete)
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
    Start.then(Code1).then(Complete)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.status == RunStatus.COMPLETED
    assert runner.run and len(runner.run.runs) == 3


async def test_run_flow_code_block(local_runtime: RuntimeHandle):
    """Run a series of code block steps."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    CodeBlock1 = Block.new(
        BlockType.ACTION,
        "Code1",
        code=code("return 2 * Input1"),
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    CodeBlock2 = Block.new(
        BlockType.ACTION,
        "Code2",
        code=code("return 3 * Input1"),
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    Code1 = Step.new(StepType.ACTION, "Code1", node=CodeBlock1)
    Code2 = Step.new(StepType.ACTION, "Code2", node=CodeBlock2)
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Code2, Complete)
    Start.then(Code1).then(
        Code2, source_port=CodeBlock1.fields.Output1, target_port=CodeBlock2.fields.Input1
    ).then(Complete)
    local_runtime.page().blocks.extend(CodeBlock1, CodeBlock2, Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, inputs={"Input1": 2})
    assert runner.outputs and runner.outputs.Output1 == 12


async def test_run_flow_race(local_runtime: RuntimeHandle):
    """Run multiple steps in parallel, losers should be aborted on completion of winner."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Race1 = Step.new(StepType.ACTION, "Race1", code=code("await asyncio.sleep(1)"))
    Race2 = Step.new(StepType.ACTION, "Race2", code=code("await asyncio.sleep(2)"))
    Race3 = Step.new(StepType.ACTION, "Race3", code=code("await asyncio.sleep(3)"))
    Flow1.steps.extend(Start, Race1, Race2, Race3, Complete)
    Start.then(Race1).then(Complete)
    Start.then(Race2).then(Complete)
    Start.then(Race3).then(Complete)
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
    Loop.then(Loop)  # infinite!
    Start.then(Loop).then(Complete)
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
    Start.then(Code1)
    Code1.then(Code1)  # infinite!
    Code1.then(Code2)
    Code2.then(Complete)
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
        mode=ActionMode.STATIC,
        code=code("await asyncio.sleep(5)"),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow.steps.extend(Start, Code1, Complete)
    Start.then(Code1).then(Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    run = Run.new(Flow)
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
