import asyncio

from bench.language import (
    Action,
    ActionType,
    Block,
    ErrorType,
    Field,
    Flow,
    FlowEdgeType,
    NodeMode,
    ProcessStatus,
    Run,
    code,
)
from bench.runtime import create_run
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_flow_empty(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Empty Code without any fields should fail."""
    Flow1 = Flow.new("Flow1")
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and not runner.tracked_run.get_children(Run)  # no nested runs


@simulated_runtime()
async def test_run_flow_lifted_from_action(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow lifted from an Action."""
    Flow1 = Flow.new("Flow1")
    Action1 = Action.new(ActionType.START, "Action1")
    Flow1.add_child(Action1)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Action1)
    assert runner.parent is not None
    assert runner.parent.tracked_run and runner.parent.tracked_run.runnable == Flow1
    assert runner.parent.tracked_run.get_children(Run)[0].runnable == Action1


@simulated_runtime()
async def test_run_flow_spurious(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Flow with Actions that go nowhere."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    End = Action.new(ActionType.END, "End")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    # don't actually connect the actions
    Flow1.add_children(Start, End, Code1)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.get_children(Run)) == 1  # just Start


@simulated_runtime()
async def test_run_flow_trivial(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Trivial flow with Start->End, no value."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    End = Action.new(ActionType.END, "End")
    Flow1.add_children(Start, End)
    Start.connect(FlowEdgeType.MANUAL, End)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.get_children(Run)) == 3


@simulated_runtime()
async def test_run_flow_create_in_test_mode(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Flow with Start->End in test mode, creating a simple Node. Should be in same node."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(Field.output("Block", Block, is_required=True),),
    )
    Start = Action.new(ActionType.START, "Start")
    Create = Action.new(
        ActionType.CODE,
        "Create",
        code=code("""\
block = Block.new(BlockType.PARAGRAPH)
Flow1.parent.add_child(block)
return {'Block': block}
"""),
        fields=(Field.output("Block", Block, is_required=True),),
    )
    End = Action.new(ActionType.END, "End")
    Flow1.add_children(Start, Create, End)
    Start.connect(FlowEdgeType.MANUAL, Create)
    Create.connect(FlowEdgeType.MANUAL, End)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, mode=NodeMode.TEST)
    assert runner.tracked_run and len(runner.tracked_run.get_children(Run)) == 5
    assert all(r.mode == NodeMode.TEST for r in runner.tracked_run.get_children(Run))
    assert runner.outputs and isinstance(runner.outputs.Block, Block)
    assert runner.outputs.Block.mode == NodeMode.TEST


@simulated_runtime()
async def test_run_flow_link_from_nowhere(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a flow with a link from nowhere. Should not be run and just be ignored."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    End = Action.new(ActionType.END, "End")
    Flow1.add_children(Start, End)
    Nowhere = Action.new(ActionType.START, "Nowhere")  # not added to flow/graph
    Nowhere.connect(FlowEdgeType.REQUIRE, End, parent=Flow1)
    Start.connect(FlowEdgeType.MANUAL, End)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.get_children(Run)) == 3


@simulated_runtime()
async def test_run_flow_link_to_nowhere(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a flow with a link to nowhere. Should not be run and just be ignored."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    End = Action.new(ActionType.END, "End")
    Nowhere = Action.new(ActionType.START, "Nowhere")  # not added to flow/graph
    Flow1.add_children(Start, Nowhere, End)
    Start.connect(FlowEdgeType.MANUAL, Nowhere, parent=Flow1)
    Start.connect(FlowEdgeType.MANUAL, End)
    runtime.page().add_child(Flow1)
    Nowhere.delete()
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.get_children(Run)) == 4


@simulated_runtime()
async def test_run_flow_force_invalid_output(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """End the flow with invalid output (should fail)."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(Field.output("Output1", str, is_required=True),),
    )
    Start = Action.new(ActionType.START, "Start")
    End = Action.new(ActionType.END, "End")
    Flow1.add_children(Start, End)
    Start.connect(FlowEdgeType.MANUAL, End)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == ProcessStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE


@simulated_runtime()
async def test_run_flow_force_invalid_input(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a action with a trigger port that forces a Run of a Action with invalid inputs (should fail)."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(Field.output("Output1", str, is_required=True),),
    )
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("return {'Output1': Input1}"),
        fields=(Field.input("Input1", str, is_required=True),),
    )
    End = Action.new(ActionType.END, "End")
    Flow1.add_children(Start, Code1, End)
    Start.connect(FlowEdgeType.MANUAL, Code1)
    Code1.connect(FlowEdgeType.MANUAL, End)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == ProcessStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE


@simulated_runtime()
async def test_run_flow_error(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a code Action that raises an error. Flow should abort and fail."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("raise ValueError"))
    Flow1.add_children(Start, Code1)
    Start.connect(FlowEdgeType.MANUAL, Code1)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == ProcessStatus.FAILED
    assert runner.tracked_run and len(runner.tracked_run.get_children(Run)) == 3


@simulated_runtime()
async def test_run_flow_abort(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a long async flow script and abort it. All pending actions should be aborted."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("await asyncio.sleep(5)"))
    End = Action.new(ActionType.END, "End")
    Flow1.add_children(Start, Code1, End)
    Start.connect(FlowEdgeType.MANUAL, Code1)
    Code1.connect(FlowEdgeType.MANUAL, End)
    runtime.page().add_child(Flow1)
    await runtime.commit()

    run, _ = create_run(Flow1, status=ProcessStatus.QUEUED, parent=runtime.main_package)
    await runtime.session.commit()
    run_task = asyncio.create_task(runtime.run_in_runtime(run, return_error=True))
    # kill after 0.5s
    await asyncio.sleep(0.5)
    runtime.runtime.stop_run(run)
    runner = await run_task
    # flow should be aborted
    assert runner.status == ProcessStatus.ABORTED
    assert runner.tracked_run
    assert runner.tracked_run.duration and runner.tracked_run.duration.total_seconds() < 1
    # inner code action should also be aborted
    assert runner.runners[2].node == Code1 and runner.runners[2].status == ProcessStatus.ABORTED
