import asyncio

from bench.language import (
    TERMINAL_RUN_STATUSES,
    Action,
    ActionType,
    Channel,
    Flow,
    Message,
    PipeType,
    Run,
    RunStatus,
    Trigger,
    code,
)
from bench.runtime import create_run_from_node
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime(runtimes=True)
async def test_run_in_runtime(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run and wait for it to execute in another Runtime."""
    Page1 = runtime.page()
    Flow1 = Flow.new("Flow1")
    Page1.append(Flow1)
    await runtime.commit()

    run = create_run_from_node(Flow1)
    await run.wait_until_terminated()
    assert run.status == RunStatus.COMPLETED


@simulated_runtime(runtimes=True)
async def test_run_flow_start_run_from_message(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Create a Run from a Message in a Flow. Should be lifted into a Flow Run."""
    Channel1 = Channel.new("General")
    Flow1 = Flow.new("Flow1")
    Receive1 = Action.new(ActionType.RECEIVE, "Receive1", triggers=[Trigger.on_message()])
    Complete1 = Action.new(ActionType.COMPLETE, "Complete1")
    Flow1.extend(Receive1, Complete1)
    runtime.page().extend(Channel1, Flow1)
    await runtime.commit()

    Message1 = Message.new(title="Hello, world!", channel=Channel1)
    runtime.bench.append(Message1)
    await runtime.commit()

    Run1 = await Run.get_run_of(Flow1, where=TERMINAL_RUN_STATUSES)
    assert Run1.status == RunStatus.COMPLETED


@simulated_runtime(runtimes=True)
async def test_run_flow_pause_resume_in_runtime(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Run a long async Flow and pause it, then resume it."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Action1 = Action.new(ActionType.CODE, "Action1", code=code("await sleep(1)"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Action1, Complete)
    Start.connect(PipeType.CALL, Action1)
    Action1.connect(PipeType.CALL, Complete)
    runtime.page().append(Flow1)
    await runtime.commit()

    async def pause_run(run: Run):
        run.pause()
        await runtime.session.commit()

    # run, pause, then resume
    run = create_run_from_node(Flow1)
    await runtime.commit()
    asyncio.get_event_loop().call_later(0.2, pause_run, run)
    await run.wait_until_status(RunStatus.PAUSED)
    run.resume()
    await runtime.commit()
    await run.wait_until_status(RunStatus.COMPLETED)
    assert run.status == RunStatus.COMPLETED
