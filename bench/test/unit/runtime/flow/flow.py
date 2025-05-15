import asyncio

from bench.language import (
    Action,
    ActionType,
    Flow,
    FlowEdgeType,
    ProcessStatus,
    code,
)
from bench.runtime import Interrupted, make_runner
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_flow_race(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run multiple actions in parallel, losers should be aborted on completion of winner."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    End = Action.new(ActionType.END, "End")
    Race1 = Action.new(ActionType.CODE, "Race1", code=code("await asyncio.sleep(1)"))
    Race2 = Action.new(ActionType.CODE, "Race2", code=code("await asyncio.sleep(2)"))
    Race3 = Action.new(ActionType.CODE, "Race3", code=code("await asyncio.sleep(3)"))
    Flow1.actions.extend(Start, Race1, Race2, Race3, End)
    Start.connect(FlowEdgeType.MANUAL, Race1)
    Start.connect(FlowEdgeType.MANUAL, Race2)
    Start.connect(FlowEdgeType.MANUAL, Race3)
    Race1.connect(FlowEdgeType.MANUAL, End)
    Race2.connect(FlowEdgeType.MANUAL, End)
    Race3.connect(FlowEdgeType.MANUAL, End)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run
    aborted_runs = [r for r in runner.tracked_run.runs if r.status == ProcessStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers should be aborted
    assert len(runner.tracked_run.runs) == 9  # all actions & links should run exactly once


@simulated_runtime(system=True)
async def test_run_flow_pause_resume(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a long async Flow and pause it, then resume it."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Action1 = Action.new(ActionType.CODE, "Action1", code=code("await sleep(0.2)"))
    Action2 = Action.new(ActionType.CODE, "Action2", code=code("await sleep(0.2)"))
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Action1, Action2, End)
    Start.connect(FlowEdgeType.MANUAL, Action1)
    Action1.connect(FlowEdgeType.MANUAL, Action2)
    Action2.connect(FlowEdgeType.MANUAL, End)
    runtime.page().append(Flow1)
    await runtime.commit()

    # run, pause, then resume (manually)
    runner = make_runner(runtime.runtime, Flow1, run="track")
    assert runner.tracked_run
    asyncio.get_event_loop().call_later(0.1, runner.tracked_run.pause)
    try:
        _ = await runtime.runtime.run_runner(runner)
    except Interrupted:
        assert runner.status == ProcessStatus.PAUSED
    runner.tracked_run.resume(_trigger_runtime=False)
    _ = await runtime.runtime.run_runner(runner)
    assert runner.status == ProcessStatus.COMPLETED
