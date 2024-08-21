from bench.language.block import Block
from bench.language.code import code
from bench.language.const import BlockType, RunStatus
from bench.language.field import Field
from bench.language.step import PipeFilterType, PortType, Step, StepType
from bench.test.unit.conftest import RuntimeHandle


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
    Start = Step.new(StepType.START, "Start")
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
    Start.then(Complete)
    Flow1.steps.extend(Start, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 2  # two steps


async def test_run_flow_force_trigger_invalid_value(local_runtime: RuntimeHandle):
    """Run a step with a trigger port that is forced to be invalid."""
    # nocheckin ...


async def test_run_flow_code(local_runtime: RuntimeHandle):
    """Run a code step with values."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.input("Input1", int), Field.output("Output1", int))
    )
    Start = Step.new(StepType.START, "Start", inputs={"Input1": 2})
    Code1 = Step.new(
        StepType.CODE,
        "Code1",
        code=code("return 2 * Input1"),
        fields=(Field.input("Input1", int), Field.output("Output1", int)),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Start.then(Code1).then(Complete)
    Flow1.steps.extend(Start, Code1, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, inputs={"Input1": 2})
    assert runner.run and len(runner.run.runs) == 3  # three steps
    assert runner.outputs and runner.outputs.Output1 == 4


async def test_run_flow_code_with_error(local_runtime: RuntimeHandle):
    """Run a code step with an error."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(StepType.CODE, "Code1", code=code("raise ValueError"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Start.then(Code1).then(Complete)
    Flow1.steps.extend(Start, Code1, Complete)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 2  # two steps


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
        fields=(Field.input("Input1", int), Field.output("Output1", int)),
    )
    Code1 = Step.new(StepType.BLOCK, "Code1", node=CodeBlock1)
    Code2 = Step.new(StepType.BLOCK, "Code2", node=CodeBlock2)
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Start.then(Code1).then(
        Code2, source_port=CodeBlock1.fields.Output1, target_port=CodeBlock2.fields.Input1
    ).then(Complete)
    Flow1.steps.extend(Start, Code1, Code2, Complete)
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
    Start.then(Race1).then(Complete)
    Race2 = Step.new(StepType.CODE, "Race2", code=code("await asyncio.sleep(2)"))
    Start.then(Race2).then(Complete)
    Race3 = Step.new(StepType.CODE, "Race3", code=code("await asyncio.sleep(3)"))
    Start.then(Race3).then(Complete)
    Flow1.steps.extend(Start, Race1, Race2, Race3, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run
    aborted_runs = [r for r in runner.run.runs if r.status == RunStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers should be aborted
    assert len(runner.run.runs) == 5  # all steps should run exactly once


async def test_run_flow_not_so_infinite_loop(local_runtime: RuntimeHandle):
    """Runs an infinite loop that's not infinite because it also completes immediately."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Loop = Step.new(StepType.CODE, "Loop")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Loop.then(Loop)  # infinite!
    Start.then(Loop).then(Complete)
    Flow1.steps.extend(Start, Loop, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.run and len(runner.run.runs) == 4  # two steps


async def test_run_flow_ping_pong(local_runtime: RuntimeHandle):
    """Run steps that pass a number back and forth until it reaches 0."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.input("NumIn", int), Field.output("NumOut", int))
    )
    Start = Step.new(StepType.START, "Start")
    Generator = Step.new(
        StepType.CODE,
        "Generator",
        fields=(Field.input("Seed", int), Field.output("Num", int)),
        code=code("return Seed - 1"),
    )
    Verifier = Step.new(
        StepType.CODE,
        "Verifier",
        fields=(Field.input("Num", int), Field.output("IsGood", bool)),
        code=code("return Num == 1"),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    # Start.Num -> Generator.Seed
    Start.then(Generator, source_port=Flow1.fields.NumIn, target_port=Generator.fields.Seed)
    # Generator.Seed > Generator.Num
    Generator.with_(Generator, source_port=Generator.fields.Num, target_port=Generator.fields.Num)
    # Generator.Num -> Verifier.Num
    Generator.then(Verifier, source_port=Generator.fields.Num, target_port=Verifier.fields.Num)
    # Generator.Num > Complete
    Generator.with_(Complete, source_port=Generator.fields.Num, target_port=Flow1.fields.NumOut)
    # Generator.IsGood -> Complete.#
    Verifier.then(
        Complete,
        source_port=Verifier.fields.IsGood,
        target_port=PortType.TRIGGER,
        filter_type=PipeFilterType.IS_TRUTHY,
    )
    Flow1.steps.extend(Start, Generator, Verifier, Complete)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, inputs={"NumIn": 5})
    assert runner.outputs and runner.outputs.NumOut == 1


# nocheckin: flow yield/halt & replay
