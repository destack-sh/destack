import asyncio

from bench.language import (
    Action,
    ActionType,
    Breakpoint,
    BreakpointScope,
    ErrorType,
    Flow,
    LinkType,
    RunOptions,
    RunStatus,
    Trigger,
    code,
)
from bench.runtime import Interrupted, create_run, make_runner
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_flow_race(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run multiple actions in parallel, losers should be aborted on completion of winner."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    End = Action.new(ActionType.END, "End")
    Race1 = Action.new(ActionType.CODE, "Race1", code=code("await asyncio.sleep(1)"))
    Race2 = Action.new(ActionType.CODE, "Race2", code=code("await asyncio.sleep(2)"))
    Race3 = Action.new(ActionType.CODE, "Race3", code=code("await asyncio.sleep(3)"))
    Flow1.actions.extend(Start, Race1, Race2, Race3, End)
    Start.connect(LinkType.MANUAL, Race1)
    Start.connect(LinkType.MANUAL, Race2)
    Start.connect(LinkType.MANUAL, Race3)
    Race1.connect(LinkType.MANUAL, End)
    Race2.connect(LinkType.MANUAL, End)
    Race3.connect(LinkType.MANUAL, End)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    aborted_runs = [r for r in runner.tracked_run.runs if r.status == RunStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers should be aborted
    assert len(runner.tracked_run.runs) == 9  # all actions & links should run exactly once


@simulated_runtime(system=True)
async def test_run_flow_yield(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with a Yield action, then resume from the Yield."""
    from bench.builtin import ActionKit

    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Yield = Action.new(ActionKit.actions.Yield, "Yield")
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Yield, End)
    Start.connect(LinkType.MANUAL, Yield)
    Yield.connect(LinkType.MANUAL, End)
    runtime.page().append(Flow1)
    await runtime.commit()

    # run up to yield
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interruption
    assert not runner.tracked_run.terminated_at
    assert len(runner.attempts) == 1

    # resume run (without handling Interruption)
    runner = await runtime.run_in_runtime(runner.tracked_run)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interruption
    assert not runner.tracked_run.terminated_at
    assert len(runner.attempts) == 1  # should be the same attempt

    # handle interruption
    runner.tracked_run.interruption.complete()

    # resume run (after handling Interruption)
    runner = await runtime.run_in_runtime(runner.tracked_run)
    assert runner.status == RunStatus.COMPLETED
    assert runner.tracked_run
    assert len(runner.attempts) == 1


@simulated_runtime(system=True)
async def test_run_flow_yield_nested(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a FLow inside another Flow and yield from there. Should propagate and resume properly."""
    from bench.builtin import ActionKit

    # inner flow
    FlowInner = Flow.new("FlowInner")
    StartInner = Action.new(ActionType.START, "StartInner", triggers=[Trigger.on_start()])
    YieldInner = Action.new(ActionKit.actions.Yield, "YieldInner")
    EndInner = Action.new(ActionType.END, "EndInner")
    FlowInner.actions.extend(StartInner, YieldInner, EndInner)
    StartInner.connect(LinkType.MANUAL, YieldInner)
    YieldInner.connect(LinkType.MANUAL, EndInner)

    # outer flow
    FlowOuter = Flow.new("FlowOuter")
    StartOuter = Action.new(ActionType.START, "StartOuter", triggers=[Trigger.on_start()])
    ActionOuter = Action.new(ActionType.TOOL, "Action", tool=FlowInner)
    EndOuter = Action.new(ActionType.END, "EndOuter")
    FlowOuter.actions.extend(StartOuter, ActionOuter, EndOuter)
    StartOuter.connect(LinkType.MANUAL, ActionOuter)
    ActionOuter.connect(LinkType.MANUAL, EndOuter)

    runtime.page().extend(FlowInner, FlowOuter)
    await runtime.commit()

    # run up to yield
    runner = await runtime.run_in_runtime(FlowOuter)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run

    # resume run (without handling Interruption)
    runner = await runtime.run_in_runtime(runner.tracked_run)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interruption

    # handle interruption
    runner.tracked_run.interruption.complete(_trigger_runtime=False)

    # resume run (after handling Interruption)
    runner = await runtime.run_in_runtime(runner.tracked_run)
    assert runner.status == RunStatus.COMPLETED


@simulated_runtime(system=True)
async def test_run_flow_yield_cancelled(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with a Yield action, then cancel it."""
    from bench.builtin import ActionKit

    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Yield = Action.new(ActionKit.actions.Yield, "Yield")
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Yield, End)
    Start.connect(LinkType.MANUAL, Yield)
    Yield.connect(LinkType.MANUAL, End)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interruption
    runner.tracked_run.interruption.cancel(_trigger_runtime=False)
    runner = await runtime.run_in_runtime(runner.tracked_run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INTERRUPTION_CANCELLED


@simulated_runtime(system=True)
async def test_run_flow_breakpoint(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with breakpoints all over. Should yield and resume properly."""
    from bench.builtin import ActionKit

    Flow1 = Flow.new(
        "Flow1",
        options=RunOptions(breakpoints=[Breakpoint.before(BreakpointScope.ACTION)]),
    )
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Yield = Action.new(
        ActionKit.actions.Yield, "Yield", options=RunOptions(breakpoints=[Breakpoint.before()])
    )
    Action1 = Action.new(
        ActionType.CODE,
        "Action1",
        code=code("pass"),
        options=RunOptions(
            breakpoints=[Breakpoint.before(), Breakpoint.after_completed(), Breakpoint.after()]
        ),
    )
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Yield, Action1, End)
    StartToYield = Start.connect(
        LinkType.MANUAL,
        Yield,
        options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_failed()]),
    )
    Yield.connect(LinkType.MANUAL, Action1)
    Action1ToEnd = Action1.connect(
        LinkType.MANUAL,
        End,
        options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_completed()]),
    )
    runtime.page().append(Flow1)
    await runtime.commit()

    # check that all yield points are hit in order
    run, _ = create_run(Flow1, status=RunStatus.QUEUED, parent=runtime.main_package)
    await runtime.session.commit()
    runner = None
    for yield_point in (
        Start,
        StartToYield,
        Yield,
        Yield,
        Action1,
        Action1,
        Action1ToEnd,
        Action1ToEnd,
        End,
    ):
        # run up to yield
        runner = await runtime.run_in_runtime(run)
        assert runner.status == RunStatus.YIELDED
        assert runner.interruption and runner.interruption.is_in(yield_point)
        assert runner.tracked_run
        run = runner.tracked_run

        # run up to yield again (without handling Interruption)
        runner = await runtime.run_in_runtime(run)
        assert runner.status == RunStatus.YIELDED
        assert runner.interruption and runner.interruption.is_in(yield_point)
        assert runner.tracked_run
        run = runner.tracked_run

        # handle interruption
        runner.interruption.complete(_trigger_runtime=False)  # manual

    # run up to completion
    runner = await runtime.run_in_runtime(run)
    assert runner.status == RunStatus.COMPLETED


@simulated_runtime(system=True)
async def test_run_flow_pause_resume(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a long async Flow and pause it, then resume it."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Action1 = Action.new(ActionType.CODE, "Action1", code=code("await sleep(0.2)"))
    Action2 = Action.new(ActionType.CODE, "Action2", code=code("await sleep(0.2)"))
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Action1, Action2, End)
    Start.connect(LinkType.MANUAL, Action1)
    Action1.connect(LinkType.MANUAL, Action2)
    Action2.connect(LinkType.MANUAL, End)
    runtime.page().append(Flow1)
    await runtime.commit()

    # run, pause, then resume (manually)
    runner = make_runner(runtime.runtime, Flow1, run="track")
    assert runner.tracked_run
    asyncio.get_event_loop().call_later(0.1, runner.tracked_run.pause)
    try:
        _ = await runtime.runtime.run_runner(runner)
    except Interrupted:
        assert runner.status == RunStatus.PAUSED
    runner.tracked_run.resume(_trigger_runtime=False)
    _ = await runtime.runtime.run_runner(runner)
    assert runner.status == RunStatus.COMPLETED
