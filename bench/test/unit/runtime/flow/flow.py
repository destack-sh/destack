import asyncio

from bench.language import (
    Action,
    ActionType,
    Breakpoint,
    BreakpointScope,
    ErrorType,
    Field,
    Flow,
    LinkType,
    RunOptions,
    RunStatus,
    code,
)
from bench.runtime import CodeActionRunner, Interrupted, create_run_from_node, make_runner
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_flow_race(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run multiple actions in parallel, losers should be aborted on completion of winner."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Race1 = Action.new(ActionType.CODE, "Race1", code=code("await asyncio.sleep(1)"))
    Race2 = Action.new(ActionType.CODE, "Race2", code=code("await asyncio.sleep(2)"))
    Race3 = Action.new(ActionType.CODE, "Race3", code=code("await asyncio.sleep(3)"))
    Flow1.actions.extend(Start, Race1, Race2, Race3, Complete)
    Start.connect(LinkType.MANUAL, Race1)
    Start.connect(LinkType.MANUAL, Race2)
    Start.connect(LinkType.MANUAL, Race3)
    Race1.connect(LinkType.MANUAL, Complete)
    Race2.connect(LinkType.MANUAL, Complete)
    Race3.connect(LinkType.MANUAL, Complete)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    aborted_runs = [r for r in runner.tracked_run.runs if r.status == RunStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers should be aborted
    assert len(runner.tracked_run.runs) == 9  # all actions & links should run exactly once


@simulated_runtime()
async def test_run_flow_call_none(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Runs a Flow with no calls selected."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Route = Action.new(ActionType.CODE, "Router", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Route, Code2, Code3, Complete)
    Start.connect(LinkType.MANUAL, Route)
    Route.connect(LinkType.AUTO, Complete)
    Route.connect(LinkType.AUTO, Code2)
    Route.connect(LinkType.AUTO, Code3)
    runtime.page().append(Flow1)
    await runtime.commit()

    Route.code = code("""
return {
    "plans": [call_none()],
}
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert not runner.tracked_run.has(Code2, Code3, Complete)


@simulated_runtime()
async def test_run_flow_call_tool(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with a tool call."""
    Flow1 = Flow.new("Flow1", fields=[Field.input("Input1", str), Field.output("Output1", str)])
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    Tool1 = Action.new(ActionType.TOOL, "Tool1")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Code1, Tool1, Complete)
    Start.connect(LinkType.MANUAL, Code1)
    Code1.connect(LinkType.MANUAL, Tool1)
    Tool1.connect(LinkType.MANUAL, Complete)
    runtime.page().append(Flow1)
    await runtime.commit()

    # run tool action directly
    runner = await runtime.run_in_runtime(
        Tool1,
        inputs={"type": ActionType.CODE, "code": code("pass")},
    )
    assert len(runner.runners) == 1
    assert isinstance(runner.runners[0], CodeActionRunner)
    assert runner.runners[0].inputs.code == code("pass")

    # running flow as is should fail (at tool, because tool is unset)
    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.RUN_IMPOSSIBLE

    # run tool within flow via calls
    Code1.code = code("""\
return {
    "plans": [call(Tool1, type=ActionType.CODE, code=code("pass"))],
}
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run


@simulated_runtime()
async def test_run_flow_call_route(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Runs a Flow with some basic routing plans."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Route = Action.new(ActionType.CODE, "Router", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    Code4 = Action.new(ActionType.CODE, "Code4", code=code("pass"))
    Code5 = Action.new(ActionType.CODE, "Code5", code=code("pass"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Route, Code2, Code3, Code4, Code5, Complete)
    Start.connect(LinkType.MANUAL, Route)
    Route.connect(LinkType.AUTO, Complete)
    Route.connect(LinkType.AUTO, Code2)
    Route.connect(LinkType.AUTO, Code3)
    Route.connect(LinkType.AUTO, Code4)
    runtime.page().append(Flow1)
    await runtime.commit()

    # Route: Code2, Code3git st
    Route.code = code("""\
return {
    "plans": [call_serial(call(Code2), call(Code3))],
}
    """)
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code2)
    assert runner.tracked_run.has(Code3)
    assert not runner.tracked_run.has(Complete)
    assert not runner.tracked_run.has(Code4)

    # Route: Code4
    Route.code = code("""\
return {
    "plans": [call_serial(call(Code4))],
}
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code4)
    assert not runner.tracked_run.has(Complete)
    assert not runner.tracked_run.has(Code2)
    assert not runner.tracked_run.has(Code3)

    # Route: Code2 & Complete
    Route.code = code("""\
return {
    "plans": [call_parallel(call(Code2), call(Complete))],
}
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(Complete)
    assert runner.tracked_run.has(Code2)
    assert not runner.tracked_run.has(Code3)
    assert not runner.tracked_run.has(Code4)

    # Route: Code5 (is not connected, so should be skipped)
    Route.code = code("""\
return {
    "plans": [call_serial(call(Code5))],
}
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert not runner.tracked_run.has(Code5)


@simulated_runtime()
async def test_run_flow_call_plan(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with more complex call plans."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Plan1 = Action.new(ActionType.CODE, "Plan1", code=code("pass"))
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    Code4 = Action.new(ActionType.CODE, "Code4", code=code("pass"))
    Fail = Action.new(ActionType.FAIL, "Fail")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Plan1, Code1, Code2, Code3, Code4, Fail, Complete)
    Start.connect(LinkType.MANUAL, Plan1)
    # Plan1 -?> Code1, Code2, Code3, Code4, Complete, Fail
    Plan1.connect(LinkType.AUTO, Code1)
    Plan1.connect(LinkType.AUTO, Code2)
    Plan1.connect(LinkType.AUTO, Code3)
    Plan1.connect(LinkType.AUTO, Code4)
    Plan1.connect(LinkType.AUTO, Complete)
    Plan1.connect(LinkType.AUTO, Fail)
    runtime.page().append(Flow1)
    await runtime.commit()

    # Plan: Code1 + Code1, Code2 + Code2
    Plan1.code = code("""\
return {
    "plans": [
        call_serial(call(Code1), call(Code1)),
        call_serial(call(Code2), call(Code2)),
        call_parallel(call(Code1), call(Code2)),
    ],
}
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert len(runner.tracked_run.get_runs(Code1)) == 3
    assert len(runner.tracked_run.get_runs(Code2)) == 3

    # Plan: Code1, Code2, Fail, Code3
    #  -> Code3 should be skipped after fail
    Plan1.code = code("""\
return {
    "plans": [
        call_serial(call(Code1), call(Code2), call(Fail), call(Code3), on_error=CallFailureMode.COMPLETE),
    ],
}
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code1)
    assert runner.tracked_run.has(Code2)
    assert runner.tracked_run.has(Fail)
    assert not runner.tracked_run.has(Code3)

    # Plan: Code1, Complete (return on terminate)
    # -> should terminate (and not loop endlessly..)
    Plan1.code = code("""\
return {
    "plans": [call_serial(call(Code1), call(Complete), on_terminate=CallTerminationMode.RETURN)],
}
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(Complete)
    assert runner.tracked_run.has(Code1)


@simulated_runtime()
async def test_run_flow_yield(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with a Yield action, then resume from the Yield."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Yield = Action.new(ActionType.YIELD, "Yield")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Yield, Complete)
    Start.connect(LinkType.MANUAL, Yield)
    Yield.connect(LinkType.MANUAL, Complete)
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


@simulated_runtime()
async def test_run_flow_yield_nested(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a FLow inside another Flow and yield from there. Should propagate and resume properly."""
    # inner flow
    FlowInner = Flow.new("FlowInner")
    StartInner = Action.new(ActionType.START, "StartInner")
    YieldInner = Action.new(ActionType.YIELD, "YieldInner")
    CompleteInner = Action.new(ActionType.COMPLETE, "CompleteInner")
    FlowInner.actions.extend(StartInner, YieldInner, CompleteInner)
    StartInner.connect(LinkType.MANUAL, YieldInner)
    YieldInner.connect(LinkType.MANUAL, CompleteInner)

    # outer flow
    FlowOuter = Flow.new("FlowOuter")
    StartOuter = Action.new(ActionType.START, "Start")
    ActionOuter = Action.new(ActionType.TOOL, "Action", tool=FlowInner)
    CompleteOuter = Action.new(ActionType.COMPLETE, "Complete")
    FlowOuter.actions.extend(StartOuter, ActionOuter, CompleteOuter)
    StartOuter.connect(LinkType.MANUAL, ActionOuter)
    ActionOuter.connect(LinkType.MANUAL, CompleteOuter)

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


@simulated_runtime()
async def test_run_flow_yield_cancelled(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with a Yield action, then cancel it."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Yield = Action.new(ActionType.YIELD, "Yield")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Yield, Complete)
    Start.connect(LinkType.MANUAL, Yield)
    Yield.connect(LinkType.MANUAL, Complete)
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


@simulated_runtime()
async def test_run_flow_breakpoint(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with breakpoints all over. Should yield and resume properly."""
    Flow1 = Flow.new(
        "Flow1",
        run_options=RunOptions(breakpoints=[Breakpoint.before(BreakpointScope.ACTION)]),
    )
    Start = Action.new(ActionType.START, "Start")
    Yield = Action.new(
        ActionType.YIELD, "Yield", run_options=RunOptions(breakpoints=[Breakpoint.before()])
    )
    Action1 = Action.new(
        ActionType.CODE,
        "Action1",
        code=code("pass"),
        run_options=RunOptions(
            breakpoints=[Breakpoint.before(), Breakpoint.after_completed(), Breakpoint.after()]
        ),
    )
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Yield, Action1, Complete)
    StartToYield = Start.connect(
        LinkType.MANUAL,
        Yield,
        run_options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_failed()]),
    )
    Yield.connect(LinkType.MANUAL, Action1)
    Action1ToComplete = Action1.connect(
        LinkType.MANUAL,
        Complete,
        run_options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_completed()]),
    )
    runtime.page().append(Flow1)
    await runtime.commit()

    # check that all yield points are hit in order
    run = create_run_from_node(Flow1, isolate=True)
    runner = None
    for yield_point in (
        Start,
        StartToYield,
        Yield,
        Yield,
        Action1,
        Action1,
        Action1ToComplete,
        Action1ToComplete,
        Complete,
    ):
        # run up to yield
        runner = await runtime.run_in_runtime(run)
        assert runner.status == RunStatus.YIELDED
        assert runner.interruption and runner.interruption.runnable == yield_point
        assert runner.tracked_run
        run = runner.tracked_run

        # run up to yield again (without handling Interruption)
        runner = await runtime.run_in_runtime(run)
        assert runner.status == RunStatus.YIELDED
        assert runner.interruption and runner.interruption.runnable == yield_point
        assert runner.tracked_run
        run = runner.tracked_run

        # handle interruption
        runner.interruption.complete(_trigger_runtime=False)  # manual

    # run up to completion
    runner = await runtime.run_in_runtime(run)
    assert runner.status == RunStatus.COMPLETED


@simulated_runtime()
async def test_run_flow_pause_resume(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a long async Flow and pause it, then resume it."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Action1 = Action.new(ActionType.CODE, "Action1", code=code("await sleep(0.2)"))
    Action2 = Action.new(ActionType.CODE, "Action2", code=code("await sleep(0.2)"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Action1, Action2, Complete)
    Start.connect(LinkType.MANUAL, Action1)
    Action1.connect(LinkType.MANUAL, Action2)
    Action2.connect(LinkType.MANUAL, Complete)
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
