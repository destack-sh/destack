import asyncio

from bench.language import Agency, BlockType, RunStatus
from bench.language.block import Block
from bench.language.code import code
from bench.language.const import NodeMode
from bench.language.field import Field
from bench.language.flow import ActionStep, PipeType, Step, StepType
from bench.language.interrupt import Breakpoint, BreakpointScope, Interrupt, InterruptStatus
from bench.language.message import Message
from bench.language.run import RunErrorType, RunOptions
from bench.language.text import Text
from bench.runtime.runner import Interrupted, make_run_from_node, make_runner
from bench.test.unit.conftest import RuntimeHandle


async def test_run_step_directly(local_runtime: RuntimeHandle):
    """Run a Step directly."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code = Step.new(StepType.ACTION, "Code", agency=Agency.CODE, code=code("pass"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Code, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    _ = await local_runtime.run(Start)
    _ = await local_runtime.run(Code)
    _ = await local_runtime.run(Complete)


async def test_run_pipe_directly(local_runtime: RuntimeHandle):
    """Run a Pipe directly."""
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
    assert runner.tracked_run and not runner.tracked_run.runs  # no nested runs


async def test_run_flow_spurious(local_runtime: RuntimeHandle):
    """Flow with Steps that go nowhere."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Flow1.steps.append(Step.new(StepType.START, "Start"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Code1 = Step.new(StepType.ACTION, "Code1", agency=Agency.CODE, code=code("pass"))
    # don't actually connect the steps
    Flow1.steps.extend(Start, Complete, Code1)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 1  # just Start


async def test_run_flow_trivial(local_runtime: RuntimeHandle):
    """Trivial flow with Start->Complete, no value."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Complete)
    Start.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_create_test_mode(local_runtime: RuntimeHandle):
    """Flow with Start->Complete in test mode, creating a simple Node."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(Field.output("Block", Block, is_required=True),),
    )
    Start = Step.new(StepType.START, "Start")
    Create = Step.new(
        StepType.ACTION,
        "Create",
        agency=Agency.CODE,
        code=code("""\
return Block.new(BlockType.TEXT, "Test")
"""),
        fields=(Field.output("Block", Block, is_required=True),),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Create, Complete)
    Start.connect(PipeType.PASS, Create)
    Create.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, mode=NodeMode.TEST)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 5
    assert all(r.mode == NodeMode.TEST for r in runner.tracked_run.runs)
    assert runner.outputs and isinstance(runner.outputs.Block, Block)
    assert runner.outputs.Block.mode == NodeMode.TEST


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
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_pipe_to_nowhere(local_runtime: RuntimeHandle):
    """Run a flow with a pipe to nowhere. Should not be run and just be ignored."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Nowhere = Step.new(StepType.START, "Nowhere")  # not added to flow/graph
    Flow1.steps.extend(Start, Complete)
    Start.connect(PipeType.PASS, Nowhere, parent=Flow1)
    Start.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 5


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
        StepType.ACTION,
        "Code1",
        agency=Agency.CODE,
        fields=(Field.input("Input1", str, is_required=True),),
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


async def test_run_flow_code_step(local_runtime: RuntimeHandle):
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
        agency=Agency.CODE,
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
    assert runner.tracked_run and len(runner.tracked_run.runs) == 5
    assert runner.outputs and runner.outputs.Output1 == 4


async def test_run_flow_error(local_runtime: RuntimeHandle):
    """Run a code Step that raises an error. Flow should abort and fail."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(StepType.ACTION, "Code1", agency=Agency.CODE, code=code("raise ValueError"))
    Flow1.steps.extend(Start, Code1)
    Start.connect(PipeType.PASS, Code1)
    local_runtime.page().blocks.extend(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_error_with_error_suppressed(local_runtime: RuntimeHandle):
    """Run a code step with an error with failure suppressed. Flow should complete."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.ACTION,
        "Code1",
        agency=Agency.CODE,
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
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_fail_step(local_runtime: RuntimeHandle):
    """Run a Flow with a Fail step."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Fail = Step.new(
        StepType.FAIL, "Fail", error_title="Fail title", error_text=Text.plain("Fail text")
    )
    Flow1.steps.extend(Start, Fail)
    Start.connect(PipeType.PASS, Fail)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    # flow
    runner = await local_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error
    assert runner.error.title == "Fail title"
    assert runner.error.text == Text.plain("Fail text")

    # run directly with custom inputs
    runner = await local_runtime.run(
        Fail,
        inputs={"error_title": "Custom title", "error_text": Text.plain("Custom text")},
        return_error=True,
    )
    assert runner.status == RunStatus.FAILED
    assert runner.error
    assert runner.error.title == "Custom title"
    assert runner.error.text == Text.plain("Custom text")


async def test_run_flow_create_step(local_runtime: RuntimeHandle):
    """Run a CreateStep to create a Message."""
    MessageType1 = Block.new(BlockType.MESSAGE, "Message", fields=(Field.member("Rating", int),))
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Create = Step.new(
        StepType.CREATE,
        "Create",
        node_partial=Message.partial(title="My message", block_base=MessageType1, Rating=2),
    )
    Flow1.steps.append(Create)
    local_runtime.page().blocks.extend(MessageType1, Flow1)
    await local_runtime.commit()

    # run from step
    runner = await local_runtime.run(Create)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 1
    assert runner.outputs and isinstance(runner.outputs, Message)
    assert runner.outputs.Rating == 2

    # run from step inputs
    runner = await local_runtime.run(Create, inputs={"Rating": 3})
    assert runner.tracked_run and len(runner.tracked_run.runs) == 1
    assert runner.outputs and isinstance(runner.outputs, Message)
    assert runner.outputs.Rating == 3


async def test_run_flow_race(local_runtime: RuntimeHandle):
    """Run multiple steps in parallel, losers should be aborted on completion of winner."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Race1 = Step.new(
        StepType.ACTION, "Race1", agency=Agency.CODE, code=code("await asyncio.sleep(1)")
    )
    Race2 = Step.new(
        StepType.ACTION, "Race2", agency=Agency.CODE, code=code("await asyncio.sleep(2)")
    )
    Race3 = Step.new(
        StepType.ACTION, "Race3", agency=Agency.CODE, code=code("await asyncio.sleep(3)")
    )
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
    assert runner.tracked_run
    aborted_runs = [r for r in runner.tracked_run.runs if r.status == RunStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers should be aborted
    assert len(runner.tracked_run.runs) == 9  # all steps & pipes should run exactly once


async def test_run_flow_infinite_loop(local_runtime: RuntimeHandle):
    """Runs an infinite loop that's not infinite because it also completes immediately."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Loop = Step.new(StepType.ACTION, "Loop", agency=Agency.CODE, code=code("pass"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.extend(Start, Loop, Complete)
    Start.connect(PipeType.PASS, Loop)
    Loop.connect(PipeType.PASS, Loop)  # infinite!
    Loop.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) <= 10  # should complete quickly


async def test_run_flow_select_continuations_manually(local_runtime: RuntimeHandle):
    """Runs a Flow with manually selected Continuations."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Router = Step.new(
        ActionStep,
        "Router",
        agency=Agency.CODE,
        code=code("pass"),
    )
    Code2 = Step.new(ActionStep, "Code2", agency=Agency.CODE, code=code("pass"))
    Code3 = Step.new(ActionStep, "Code3", agency=Agency.CODE, code=code("pass"))
    Code4 = Step.new(ActionStep, "Code4", agency=Agency.CODE, code=code("pass"))
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow.steps.extend(Start, Router, Code2, Code3, Code4, Complete)
    Start.connect(PipeType.PASS, Router)
    Router.connect(PipeType.SELECT, Complete)
    Router.connect(PipeType.SELECT, Code2)
    Router.connect(PipeType.OPTION, Code3)
    Router.connect(PipeType.OPTION, Code4)
    await local_runtime.commit()

    # Route: none
    Router.code = code("pass")
    runner = await local_runtime.run(Flow)
    assert runner.tracked_run
    assert not runner.tracked_run.has(Code2, Code3, Code4, Complete)

    # Route: Code2 (Select) + Code3 (Option)
    Router.code = code("""\
return {
    'continuations': [Continue.new(Code2), Continue.new(Code3)],
}
""")
    runner = await local_runtime.run(Flow)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code2, Code3)
    assert not runner.tracked_run.has(Complete, Code4)

    # Route: Code4 (Option)
    Router.code = code("""\
return {
    'continuations': [Continue.new(Code4)],
}
""")
    runner = await local_runtime.run(Flow)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code4)
    assert not runner.tracked_run.has(Complete, Code2, Code3)

    # Route: Complete (Select) + Code2 (Select)
    Router.code = code("""\
return {
    'continuations': [Continue.new(Complete), Continue.new(Code2)],
}
""")
    runner = await local_runtime.run(Flow)
    assert runner.tracked_run
    assert runner.tracked_run.has(Complete, Code2)
    assert not runner.tracked_run.has(Code3, Code4)

    # Route: Code3 (Option) + Code4 (Option) -> invalid (two mutually exclusive options)
    Router.code = code("""\
return {
    'continuations': [Continue.new(Code3), Continue.new(Code4)],
}
""")
    runner = await local_runtime.run(Flow, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INVALID_CONTINUATION


async def test_run_flow_abort(local_runtime: RuntimeHandle):
    """Run a long async flow script and abort it. All pending steps should be aborted."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Code1 = Step.new(
        StepType.ACTION,
        "Code1",
        agency=Agency.CODE,
        code=code("await asyncio.sleep(5)"),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow.steps.extend(Start, Code1, Complete)
    Start.connect(PipeType.PASS, Code1)
    Code1.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    run = make_run_from_node(Flow)
    run_task = asyncio.create_task(local_runtime.run(run, return_error=True))
    # kill after 0.5s
    await asyncio.sleep(0.5)
    local_runtime.runtime.stop(run)
    runner = await run_task
    # flow should be aborted
    assert runner.status == RunStatus.ABORTED
    assert runner.tracked_run
    assert runner.tracked_run.duration and runner.tracked_run.duration.total_seconds() < 1
    # inner code step should also be aborted
    assert runner.runners[2].node == Code1 and runner.runners[2].status == RunStatus.ABORTED


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

    # run up to yield
    runner = await local_runtime.run(Flow)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interrupt
    assert not runner.tracked_run.terminated_at and not runner.tracked_run.terminated_epoch
    assert len(runner.attempts) == 1

    # resume run (without handling Interrupt)
    runner = await local_runtime.run(runner.tracked_run)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interrupt
    assert not runner.tracked_run.terminated_at and not runner.tracked_run.terminated_epoch
    assert len(runner.attempts) == 1  # should be the same attempt

    # handle interrupt
    runner.tracked_run.interrupt.complete()

    # resume run (after handling Interrupt)
    runner = await local_runtime.run(runner.tracked_run)
    assert runner.status == RunStatus.COMPLETED
    assert runner.tracked_run
    assert len(runner.attempts) == 1


async def test_run_flow_yield_nested(local_runtime: RuntimeHandle):
    "Run a FLow inside another Flow and yield from there. Should propagate and resume properly."
    # inner flow
    FlowInner = Block.new(BlockType.FLOW, "FlowInner")
    StartInner = Step.new(StepType.START, "StartInner")
    YieldInner = Step.new(StepType.YIELD, "YieldInner")
    CompleteInner = Step.new(StepType.COMPLETE, "CompleteInner")
    FlowInner.steps.extend(StartInner, YieldInner, CompleteInner)
    StartInner.connect(PipeType.PASS, YieldInner)
    YieldInner.connect(PipeType.PASS, CompleteInner)

    # outer flow
    FlowOuter = Block.new(BlockType.FLOW, "FlowOuter")
    StartOuter = Step.new(StepType.START, "Start")
    ActionOuter = Step.new(StepType.ACTION, "Action", agency=Agency.DELEGATE, delegate=FlowInner)
    CompleteOuter = Step.new(StepType.COMPLETE, "Complete")
    FlowOuter.steps.extend(StartOuter, ActionOuter, CompleteOuter)
    StartOuter.connect(PipeType.PASS, ActionOuter)
    ActionOuter.connect(PipeType.PASS, CompleteOuter)

    local_runtime.page().blocks.extend(FlowInner, FlowOuter)
    await local_runtime.commit()

    # run up to yield
    runner = await local_runtime.run(FlowOuter)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run

    # resume run (without handling Interrupt)
    runner = await local_runtime.run(runner.tracked_run)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interrupt

    # handle interrupt
    runner.tracked_run.interrupt.complete()

    # resume run (after handling Interrupt)
    runner = await local_runtime.run(runner.tracked_run)
    assert runner.status == RunStatus.COMPLETED


async def test_run_flow_yield_cancelled(local_runtime: RuntimeHandle):
    """Run a Flow with a Yield step, then cancel it."""
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
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupt
    runner.tracked_run.interrupt.cancel()
    runner = await local_runtime.run(runner.tracked_run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INTERRUPT_CANCELLED


async def test_run_flow_breakpoint(local_runtime: RuntimeHandle):
    """Run a Flow with breakpoints all over. Should yield and resume properly."""
    Flow = Block.new(
        BlockType.FLOW,
        "Flow1",
        run_options=RunOptions(breakpoints=[Breakpoint.before(BreakpointScope.STEP)]),
    )
    Start = Step.new(StepType.START, "Start")
    Yield = Step.new(
        StepType.YIELD, "Yield", run_options=RunOptions(breakpoints=[Breakpoint.before()])
    )
    Action = Step.new(
        StepType.ACTION,
        "Action",
        agency=Agency.CODE,
        code=code("pass"),
        run_options=RunOptions(
            breakpoints=[Breakpoint.before(), Breakpoint.after_completed(), Breakpoint.after()]
        ),
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow.steps.extend(Start, Yield, Action, Complete)
    StartToYield = Start.connect(
        PipeType.PASS,
        Yield,
        run_options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_failed()]),
    )
    Yield.connect(PipeType.PASS, Action)
    ActionToComplete = Action.connect(
        PipeType.PASS,
        Complete,
        run_options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_completed()]),
    )
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    # check that all yield points are hit in order
    run = make_run_from_node(Flow)
    runner = None
    for yield_point in (
        Start,
        StartToYield,
        Yield,
        Yield,
        Action,
        Action,
        ActionToComplete,
        ActionToComplete,
        Complete,
    ):
        # run up to yield
        runner = await local_runtime.run(run)
        assert runner.status == RunStatus.YIELDED
        assert runner.interrupt and runner.interrupt.runnable == yield_point
        assert runner.tracked_run
        run = runner.tracked_run

        # run up to yield again (without handling Interrupt)
        runner = await local_runtime.run(run)
        assert runner.status == RunStatus.YIELDED
        assert runner.interrupt and runner.interrupt.runnable == yield_point
        assert runner.tracked_run
        run = runner.tracked_run

        # handle interrupt
        runner.interrupt.complete()

    # run up to completion
    runner = await local_runtime.run(run)
    assert runner.status == RunStatus.COMPLETED


async def test_run_flow_pause_resume(local_runtime: RuntimeHandle):
    """Run a long async Flow and pause it, then resume it."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Action1 = Step.new(
        StepType.ACTION, "Action1", agency=Agency.CODE, code=code("await sleep(0.2)")
    )
    Action2 = Step.new(
        StepType.ACTION, "Action2", agency=Agency.CODE, code=code("await sleep(0.2)")
    )
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow.steps.extend(Start, Action1, Action2, Complete)
    Start.connect(PipeType.PASS, Action1)
    Action1.connect(PipeType.PASS, Action2)
    Action2.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    # run, pause
    runner = make_runner(local_runtime.runtime, Flow, track=True)
    assert runner.tracked_run is not None
    asyncio.get_event_loop().call_later(0.1, runner.tracked_run.pause)
    try:
        _ = await local_runtime.runtime.run_runner(runner)
    except Interrupted:
        assert runner.status == RunStatus.PAUSED

    # resume
    runner.tracked_run.resume()
    _ = await local_runtime.runtime.run_runner(runner)
    assert runner.status == RunStatus.COMPLETED


async def test_run_flow_autoclose_interrupts(local_runtime: RuntimeHandle):
    """Run and complete a Flow with an Interrupt active, it should auto-cancel."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Yield = Step.new(StepType.YIELD, "Yield")
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Pass = Step.new(StepType.ACTION, "Pass", agency=Agency.CODE, code=code("pass"))
    Flow.steps.extend(Start, Yield, Pass, Complete)
    Start.connect(PipeType.PASS, Yield)
    Start.connect(PipeType.PASS, Pass)
    Pass.connect(PipeType.PASS, Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow)
    assert runner.status == RunStatus.COMPLETED
    assert runner.tracked_run is not None

    interrupts = runner.tracked_run._graph.nodes_of_type(Interrupt)
    assert len(interrupts) == 1
    assert interrupts[0].status == InterruptStatus.CANCELLED
