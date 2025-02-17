from bench.language import Action, ActionType, Flow, Message, PipeType, Run, RunStatus, Trigger
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
async def test_run_flow_from_message(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run from a Message in a Flow."""
    Flow1 = Flow.new("Flow1")
    Receive1 = Action.new(ActionType.RECEIVE, "Receive1", triggers=[Trigger.on_message()])
    Complete1 = Action.new(ActionType.COMPLETE, "Complete1")
    Flow1.extend(Receive1, Complete1)
    Receive1.connect(PipeType.CALL, Complete1)
    runtime.page().append(Flow1)
    await runtime.commit()

    Message1 = Message.new(title="Hello, world!")
    runtime.bench.append(Message1)
    await runtime.commit()

    Run1 = await Run.get_run_of(Receive1)
