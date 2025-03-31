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
    Start.connect(LinkType.REQUIRE, Race1, is_manual=True)
    Start.connect(LinkType.REQUIRE, Race2, is_manual=True)
    Start.connect(LinkType.REQUIRE, Race3, is_manual=True)
    Race1.connect(LinkType.REQUIRE, End, is_manual=True)
    Race2.connect(LinkType.REQUIRE, End, is_manual=True)
    Race3.connect(LinkType.REQUIRE, End, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    aborted_runs = [r for r in runner.tracked_run.runs if r.status == RunStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers should be aborted
    assert len(runner.tracked_run.runs) == 9  # all actions & links should run exactly once


@simulated_runtime()
async def test_run_flow_plan_none(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Runs a Flow with no Plans/Tasks."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Route = Action.new(ActionType.CODE, "Router", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Route, Code2, Code3, End)
    Start.connect(LinkType.REQUIRE, Route, is_manual=True)
    Route.connect(LinkType.DECIDE, End, is_manual=True)
    Route.connect(LinkType.DECIDE, Code2, is_manual=True)
    Route.connect(LinkType.DECIDE, Code3, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    Route.code = code("""
pass
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert not runner.tracked_run.has(Code2, Code3, End)


@simulated_runtime()
async def test_run_flow_plan_route(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Runs a Flow with some basic routing Plans."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Router = Action.new(ActionType.CODE, "Router", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    Code4 = Action.new(ActionType.CODE, "Code4", code=code("pass"))
    Code5 = Action.new(ActionType.CODE, "Code5", code=code("pass"))
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Router, Code2, Code3, Code4, Code5, End)
    Start.connect(LinkType.REQUIRE, Router, is_manual=True)
    Router.connect(LinkType.DECIDE, End, is_manual=True)
    Router.connect(LinkType.DECIDE, Code2, is_manual=True)
    Router.connect(LinkType.DECIDE, Code3, is_manual=True)
    Router.connect(LinkType.DECIDE, Code4, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    # Router: Code2, Code3
    Router.code = code("""\
plan = Plan.serial(
    "Plan",
    Task.run("Code2", Code2),
    Task.run("Code3", Code3),
    on_terminate=PlanTerminationMode.PASS,
)
run.plans.append(plan)
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code2)
    assert runner.tracked_run.has(Code3)
    assert not runner.tracked_run.has(End)
    assert not runner.tracked_run.has(Code4)

    # Router: Code4
    Router.code = code("""\
plan = Plan.serial(
    "Plan",
    Task.run("Code4", Code4),
    on_terminate=PlanTerminationMode.PASS,
)
run.plans.append(plan)
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code4)
    assert not runner.tracked_run.has(End)
    assert not runner.tracked_run.has(Code2)
    assert not runner.tracked_run.has(Code3)

    # Router: Code2 & End
    Router.code = code("""\
plan = Plan.parallel(
    "Plan",
    Task.run("Code2", Code2),
    Task.run("End", End),
    on_terminate=PlanTerminationMode.PASS,
)
run.plans.append(plan)
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(End)
    assert runner.tracked_run.has(Code2)
    assert not runner.tracked_run.has(Code3)
    assert not runner.tracked_run.has(Code4)

    # Router: Code5 (is not connected, so should be skipped)
    Router.code = code("""\
plan = Plan.serial(
    "Plan",
    Task.run("Code5", Code5),
    on_terminate=PlanTerminationMode.PASS,
)
run.plans.append(plan)
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert not runner.tracked_run.has(Code5)


@simulated_runtime()
async def test_run_flow_plan_multiple(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with multiple Plans."""
    from bench.builtin import ActionKit

    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Plan1 = Action.new(ActionType.CODE, "Plan1", code=code("pass"))
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    Code4 = Action.new(ActionType.CODE, "Code4", code=code("pass"))
    Fail = Action.new(ActionKit.actions.Fail, "Fail")
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Plan1, Code1, Code2, Code3, Code4, Fail, End)
    Start.connect(LinkType.REQUIRE, Plan1, is_manual=True)
    # Plan1 -?> Code1, Code2, Code3, Code4, End, Fail
    Plan1.connect(LinkType.DECIDE, Code1, is_manual=True)
    Plan1.connect(LinkType.DECIDE, Code2, is_manual=True)
    Plan1.connect(LinkType.DECIDE, Code3, is_manual=True)
    Plan1.connect(LinkType.DECIDE, Code4, is_manual=True)
    Plan1.connect(LinkType.DECIDE, End, is_manual=True)
    Plan1.connect(LinkType.DECIDE, Fail, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    # Plan: Code1 + Code1, Code2 + Code2
    Plan1.code = code("""\
plan1 = Plan.serial("Plan1", 
    Task.run("Code1", Code1),
    Task.run("Code1", Code1),
    on_terminate=PlanTerminationMode.PASS,
)
plan2 = Plan.serial("Plan2", 
    Task.run("Code2", Code2),
    Task.run("Code2", Code2),
    on_terminate=PlanTerminationMode.PASS,
)
plan3 = Plan.parallel("Plan3", 
    Task.run("Code1", Code1),
    Task.run("Code2", Code2),
    on_terminate=PlanTerminationMode.PASS,
)
run.plans.extend(plan1, plan2, plan3)
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert len(runner.tracked_run.get_runs(Code1)) == 3
    assert len(runner.tracked_run.get_runs(Code2)) == 3

    # Plan: Code1, Code2, Fail, Code3
    #  -> Code3 should be skipped after fail
    Plan1.code = code("""\
plan = Plan.serial(
    "Plan", 
    Task.run("Code1", Code1),
    Task.run("Code2", Code2),
    Task.run("Fail", Fail),
    Task.run("Code3", Code3),
    on_failure=PlanFailureMode.END,
    on_terminate=PlanTerminationMode.PASS,
)
run.plans.append(plan)
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code1)
    assert runner.tracked_run.has(Code2)
    assert runner.tracked_run.has(Fail)
    assert not runner.tracked_run.has(Code3)

    # Plan: Code1, End (return on terminate)
    # -> should terminate (and not loop endlessly..)
    Plan1.code = code("""\
plan = Plan.serial(
    "Plan", 
    Task.run("Code1", Code1),
    Task.run("End", End),
    on_terminate=PlanTerminationMode.RETURN,
)
run.plans.append(plan)
""")
    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    assert runner.tracked_run.has(End)
    assert runner.tracked_run.has(Code1)


@simulated_runtime(system=True)
async def test_run_flow_yield(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with a Yield action, then resume from the Yield."""
    from bench.builtin import ActionKit

    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Yield = Action.new(ActionKit.actions.Yield, "Yield")
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Yield, End)
    Start.connect(LinkType.REQUIRE, Yield, is_manual=True)
    Yield.connect(LinkType.REQUIRE, End, is_manual=True)
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
    StartInner.connect(LinkType.REQUIRE, YieldInner, is_manual=True)
    YieldInner.connect(LinkType.REQUIRE, EndInner, is_manual=True)

    # outer flow
    FlowOuter = Flow.new("FlowOuter")
    StartOuter = Action.new(ActionType.START, "StartOuter", triggers=[Trigger.on_start()])
    ActionOuter = Action.new(ActionType.TOOL, "Action", tool=FlowInner)
    EndOuter = Action.new(ActionType.END, "EndOuter")
    FlowOuter.actions.extend(StartOuter, ActionOuter, EndOuter)
    StartOuter.connect(LinkType.REQUIRE, ActionOuter, is_manual=True)
    ActionOuter.connect(LinkType.REQUIRE, EndOuter, is_manual=True)

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
    Start.connect(LinkType.REQUIRE, Yield, is_manual=True)
    Yield.connect(LinkType.REQUIRE, End, is_manual=True)
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
        LinkType.REQUIRE,
        Yield,
        is_manual=True,
        options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_failed()]),
    )
    Yield.connect(LinkType.REQUIRE, Action1, is_manual=True)
    Action1ToEnd = Action1.connect(
        LinkType.REQUIRE,
        End,
        is_manual=True,
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
    Start.connect(LinkType.REQUIRE, Action1, is_manual=True)
    Action1.connect(LinkType.REQUIRE, Action2, is_manual=True)
    Action2.connect(LinkType.REQUIRE, End, is_manual=True)
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
