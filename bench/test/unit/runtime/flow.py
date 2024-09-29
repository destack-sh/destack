import asyncio

from bench.language.block import Block
from bench.language.code import code
from bench.language.const import BlockType, RunStatus
from bench.language.field import Field
from bench.language.flow import PipeFilter, PipeModulation, PortType, Step, StepType
from bench.language.run import Run, RunErrorType, RunOptions
from bench.language.text import md
from bench.test.unit.conftest import RuntimeHandle


async def test_run_step_directly(local_runtime: RuntimeHandle):
    """Run a Steps directly."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code = Step.new(StepType.CODE, "Code", code=code("pass"))
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
    Code1 = Step.new(StepType.CODE, "Code1", code=code("pass"))
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
    Start.then(Complete, source_port=PortType.RUN, target_port=PortType.RUN)
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
    Code1 = Step.new(StepType.CODE, "Code1", fields=(Field.input("Input1", str, is_required=True),))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.then(Code1, source_port=PortType.RUN, target_port=PortType.RUN).then(Code1).then(Complete)
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
    Start = Step.new(StepType.START, "Start", inputs={"Input1": 2})
    Code1 = Step.new(
        StepType.CODE,
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
        StepType.CODE,
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
    """Run a code step with an error (without error port). Entire flow should abort and fail."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(StepType.CODE, "Code1", code=code("raise ValueError"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.then(Code1).then(Complete)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.run and len(runner.run.runs) == 2


async def test_run_flow_error_with_error_suppressed(local_runtime: RuntimeHandle):
    """Run a code step with an error connected to the error port. Flow should complete."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.CODE,
        "Code1",
        code=code("raise ValueError('error')"),
        run_options=RunOptions(suppress_fail=True),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.then(Code1).then(
        Complete,
        source_port=PortType.RUN,
        target_port=PortType.RUN,
        filter=PipeFilter.HAS_ERROR,
    )
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
        BlockType.CODE,
        "Code1",
        code=code("return 2 * Input1"),
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    CodeBlock2 = Block.new(
        BlockType.CODE,
        "Code2",
        code=code("return 3 * Input1"),
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    Code1 = Step.new(StepType.BLOCK, "Code1", node=CodeBlock1)
    Code2 = Step.new(StepType.BLOCK, "Code2", node=CodeBlock2)
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
    Race1 = Step.new(StepType.CODE, "Race1", code=code("await asyncio.sleep(1)"))
    Race2 = Step.new(StepType.CODE, "Race2", code=code("await asyncio.sleep(2)"))
    Race3 = Step.new(StepType.CODE, "Race3", code=code("await asyncio.sleep(3)"))
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
    Loop = Step.new(StepType.CODE, "Loop")
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
    Code1 = Step.new(StepType.CODE, "Code1")
    Code2 = Step.new(StepType.CODE, "Code2")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Code2, Complete)
    Start.then(Code1)
    Code1.then(Code1)  # infinite!
    Code1.then(Code2)
    Code2.then(Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    _ = await local_runtime.run(Flow1)


async def test_run_flow_get_run_as_field(local_runtime: RuntimeHandle):
    """Run a flow with a step that gets the Run port as a field."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.output("Duration", float, is_required=True),)
    )
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(StepType.CODE, "Code1", code=code("await asyncio.sleep(0.1)"))
    Code2 = Step.new(
        StepType.CODE,
        "Code2",
        code=code("return Run1.duration"),
        fields=(
            Field.input("Run1", Run, is_required=True),
            Field.output("Duration", float, is_required=True),
        ),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Code2, Complete)
    Start.then(Code1).then(Code2, source_port=PortType.RUN, target_port=Code2.fields.Run1).then(
        Complete
    )
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 4  # two steps
    assert (
        runner.outputs
        and isinstance(runner.outputs.Duration, float)
        and runner.outputs.Duration >= 0.1  # >= sleep in that code step (above)
    )


async def test_run_flow_pipe_filter_positive(local_runtime: RuntimeHandle):
    """Run a flow with a simple positive filters on a single field."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.CODE,
        "Code1",
        fields=(Field.output("Output1", int, is_required=True),),
        code=code("return 0"),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.then(Code1).then(
        Complete,
        source_port=Code1.fields.Output1,
        target_port=PortType.RUN,
        filter=PipeFilter.IS_TRUTHY,
    )
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 2  # Start + Code1 (not Complete)


async def test_run_flow_pipe_filter_negative(local_runtime: RuntimeHandle):
    """Run a flow with a simple negative filters on an entire object."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.CODE,
        "Code1",
        fields=(
            Field.output("Output1", int, is_list=True, is_required=True),
            Field.output("Output2", int, is_required=True),
        ),
        code=code("return [1, 2, 3], 2"),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code1, Complete)
    Start.then(Code1).then(Complete, filter=PipeFilter.IS_FALSY)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 2  # Start + Code1 (not Complete)


async def test_run_flow_pipe_filter_switch(local_runtime: RuntimeHandle):
    """Run a flow with pipe filters arranged as a switch."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    Start = Step.new(StepType.START, "Start")
    Switch = Step.new(
        StepType.CODE,
        "Swich",
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
            Field.output("IsSmall", bool, is_required=True),
            Field.output("IsLarge", bool, is_required=True),
        ),
        code=code("""\
return Input1, Input1 < 10, Input1 > 10
"""),
    )
    MultiplyLarge = Step.new(
        StepType.CODE,
        "MultiplyLarge",
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
        code=code("return Input1 * 2"),
    )
    DivideSmall = Step.new(
        StepType.CODE,
        "DivideSmall",
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
        code=code("return Input1 // 2"),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Switch, MultiplyLarge, DivideSmall, Complete)
    # Start -> Switch
    Start.then(Switch, source_port=Flow1.fields.Input1, target_port=Switch.fields.Input1)
    # Switch > MultiplyLarge
    Switch.then(
        MultiplyLarge,
        source_port=Switch.fields.IsLarge,
        target_port=PortType.RUN,
        filter=PipeFilter.IS_TRUTHY,
    )
    # Switch > DivideSmall
    Switch.then(
        DivideSmall,
        source_port=Switch.fields.IsSmall,
        target_port=PortType.RUN,
        filter=PipeFilter.IS_TRUTHY,
    )
    # Start - MultiplyLarge
    Start.to(MultiplyLarge)
    # Start - DivideSmall
    Start.to(DivideSmall)
    # MultiplyLarge -> Complete
    MultiplyLarge.then(Complete)
    # DivideSmall -> Complete
    DivideSmall.then(Complete)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    # run with 'small' input (should go via DivideSmall)
    runner = await local_runtime.run(Flow1, inputs={"Input1": 6})
    assert runner.outputs and runner.outputs.Output1 == 3

    # run with 'large' input (should go via MultiplyLarge)
    runner = await local_runtime.run(Flow1, inputs={"Input1": 12})
    assert runner.outputs and runner.outputs.Output1 == 24


async def test_run_flow_generator_verifier(local_runtime: RuntimeHandle):
    """Run steps that pass a number back and forth between generator/verifier until it passes."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(
            Field.input("NumIn", int, is_required=True),
            Field.output("NumOut", int, is_required=True),
        ),
    )
    Start = Step.new(StepType.START, "Start")
    Generator = Step.new(
        StepType.CODE,
        "Generator",
        fields=(
            Field.input("Seed", int, is_required=True),
            Field.output("Num", int, is_required=True),
        ),
        code=code("return Seed - 1"),
    )
    Verifier = Step.new(
        StepType.CODE,
        "Verifier",
        text=md("Check if Num is <= 1"),
        fields=(
            Field.input("Num", int, is_required=True),
            Field.output("IsGood", bool, is_required=True),
        ),
        code=code("return Num <= 1"),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Generator, Verifier, Complete)
    # Start.Num -> Generator.Seed
    Start.then(Generator, source_port=Flow1.fields.NumIn, target_port=Generator.fields.Seed)
    # Generator.Num - Generator.Seed
    Generator.to(Generator, source_port=Generator.fields.Num, target_port=Generator.fields.Seed)
    # Generator.Num -> Verifier.Num
    Generator.then(Verifier, source_port=Generator.fields.Num, target_port=Verifier.fields.Num)
    # Generator.Num - Complete.NumOut
    Generator.to(Complete, source_port=Generator.fields.Num, target_port=Flow1.fields.NumOut)
    # Verifier.IsGood?[IsTruthy] > Complete
    Verifier.then(
        Complete,
        source_port=Verifier.fields.IsGood,
        target_port=PortType.RUN,
        filter=PipeFilter.IS_TRUTHY,
    )
    # Verifier.IsGood?[IsFalsy] > Generator
    Verifier.then(
        Generator,
        source_port=Verifier.fields.IsGood,
        target_port=PortType.RUN,
        filter=PipeFilter.IS_FALSY,
    )
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, inputs={"NumIn": 5})
    assert runner.outputs and runner.outputs.NumOut == 1


async def test_run_flow_abort(local_runtime: RuntimeHandle):
    """Run a long async flow script and abort it. All pending steps should be aborted."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.CODE,
        "Code1",
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
    assert runner.run and runner.run.duration and runner.run.duration < 1
    # code step should also be aborted
    assert runner.runs[1].node == Code1 and runner.runs[1].status == RunStatus.ABORTED


async def test_run_flow_flatten_accumulate(local_runtime: RuntimeHandle):
    """Run a flow with a pipe with a flatten and accumulate modulation."""
    Flow = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=[
            Field.input("Strings", str, is_list=True, is_required=True),
            Field.output("Total", int),
        ],
    )
    Start = Step.new(StepType.START, "Start")
    Measure = Step.new(
        StepType.CODE,
        "Measure",
        code=code("pass return len(Input1)"),
        fields=[Field.input("String", str), Field.output("Length", int)],
    )
    Reduce = Step.new(
        StepType.CODE,
        "Reduce",
        code=code("return sum(Lengths)"),
        fields=[
            Field.input("Lengths", int, is_list=True, is_required=True),
            Field.output("Total", int),
        ],
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow.steps.extend(Start, Measure, Reduce, Complete)
    Start.then(
        Measure,
        source_port=Flow.fields.Strings,
        target_port=Measure.fields.String,
        modulation=PipeModulation.FLATTEN,
    ).then(
        Reduce,
        source_port=Measure.fields.Length,
        target_port=Reduce.fields.Lengths,
        modulation=PipeModulation.ACCUMULATE,
    ).then(Complete, source_port=Reduce.fields.Total, target_port=Flow.fields.Total)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow, inputs={"Strings": ["a", "bb", "ccc"]})
    assert runner.outputs and runner.outputs.Total == 6
