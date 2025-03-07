import asyncio

from bench.language import (
    Action,
    ActionType,
    Block,
    ComputedValueMode,
    ErrorType,
    Field,
    Flow,
    LinkType,
    NodeMode,
    PathElementType,
    Run,
    RunOptions,
    RunStatus,
    code,
)
from bench.runtime import create_run_from_node
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_flow_empty(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Empty Code without any fields should fail."""
    Flow1 = Flow.new("Flow1")
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and not runner.tracked_run.runs  # no nested runs


@simulated_runtime()
async def test_run_flow_lifted_from_action(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow lifted from an Action."""
    Flow1 = Flow.new("Flow1")
    Action1 = Action.new(ActionType.START, "Action1")
    Flow1.actions.append(Action1)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Action1)
    assert runner.parent is not None
    assert runner.parent.tracked_run and runner.parent.tracked_run.runnable == Flow1
    assert runner.parent.tracked_run.runs[0].runnable == Action1


@simulated_runtime()
async def test_run_flow_spurious(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Flow with Actions that go nowhere."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    # don't actually connect the actions
    Flow1.actions.extend(Start, Complete, Code1)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 1  # just Start


@simulated_runtime()
async def test_run_flow_trivial(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Trivial flow with Start->Complete, no value."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


@simulated_runtime()
async def test_run_flow_code(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a code action with values."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("return {'Output1': 2 * Input1}"),
        fields=(
            Field.input("Input1", int, is_required=True),
            Field.output("Output1", int, is_required=True),
        ),
    )
    Code1.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Code1.fields.Input1),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input1),
    )
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output1),
        source=(Code1, PathElementType.RUN, Run.get_property("outputs"), Code1.fields.Output1),
    )
    Flow1.actions.extend(Start, Code1, Complete)
    Start.connect(LinkType.REQUIRE, Code1, is_manual=True)
    Code1.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, inputs={"Input1": 2})
    assert runner.tracked_run and len(runner.tracked_run.runs) == 5
    assert runner.outputs and runner.outputs.Output1 == 4


@simulated_runtime()
async def test_run_flow_computed_value_chain(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Run a Flow with Actions chaining computed inputs."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(
            Field.input("BoolIn", bool),
            Field.input("IntIn", int),
            Field.output("BoolOut", bool),
            Field.output("IntOut", int),
        ),
    )
    Start = Action.new(ActionType.START, "Start")
    Flow1.actions.append(Start)
    prev = Start
    for i in range(4):
        Code = Action.new(
            ActionType.CODE,
            f"Code{i}",
            code=code("""
    return {'BoolOut': not BoolIn, 'IntOut': IntIn + 1}
    """),
            fields=(
                Field.input("BoolIn", bool),
                Field.input("IntIn", int),
                Field.output("BoolOut", bool),
                Field.output("IntOut", int),
            ),
        )
        Flow1.actions.append(Code)
        if i == 0:
            Code.set_computed(
                target=(PathElementType.RUN, Run.get_property("inputs"), Code.fields.BoolIn),
                source=(
                    Flow1,
                    PathElementType.RUN,
                    Run.get_property("inputs"),
                    Flow1.fields.BoolIn,
                ),
            )
            Code.set_computed(
                target=(PathElementType.RUN, Run.get_property("inputs"), Code.fields.IntIn),
                source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.IntIn),
            )
        else:
            Code.set_computed(
                target=(PathElementType.RUN, Run.get_property("inputs"), Code.fields.BoolIn),
                source=(
                    prev,
                    PathElementType.RUN,
                    Run.get_property("outputs"),
                    prev.fields.BoolOut,
                ),
            )
            Code.set_computed(
                target=(PathElementType.RUN, Run.get_property("inputs"), Code.fields.IntIn),
                source=(prev, PathElementType.RUN, Run.get_property("outputs"), prev.fields.IntOut),
            )
        prev.connect(LinkType.REQUIRE, Code, is_manual=True)
        prev = Code
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.BoolOut),
        source=(prev, PathElementType.RUN, Run.get_property("outputs"), prev.fields.BoolOut),
    )
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.IntOut),
        source=(prev, PathElementType.RUN, Run.get_property("outputs"), prev.fields.IntOut),
    )
    Flow1.actions.append(Complete)
    prev.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, inputs={"BoolIn": True, "IntIn": 0})
    assert runner.outputs and runner.outputs.BoolOut is True
    assert runner.outputs and runner.outputs.IntOut == 4


@simulated_runtime()
async def test_run_flow_invalid_computed_source(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Run a Flow with invalid computed values (invalid source). Should fail."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(Field.input("Input", int), Field.output("Output", int)),
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(LinkType.REQUIRE, Complete, is_manual=True)
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output),
        # missing PathElementType.RUN for source, and Block has no inputs
        source=(Flow1, Run.get_property("inputs"), Flow1.fields.Input),
    )
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_COMPUTED


@simulated_runtime()
async def test_run_flow_invalid_computed_target(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Run a Flow with an invalid computed value (invalid target). Should pass (?)."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(Field.input("Input", int), Field.output("Output", int)),
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(LinkType.REQUIRE, Complete, is_manual=True)
    Complete.set_computed(
        # refers to Output, but we delete output below (oh no!, should be ignored)
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input),
    )
    runtime.page().append(Flow1)
    await runtime.commit()

    # delete output Field
    Flow1.fields.Output.delete()
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, inputs={"Input": 1})
    assert runner.status == RunStatus.COMPLETED


@simulated_runtime()
async def test_run_flow_computed_value_mode(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with computed value set if source is set."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(
            Field.input("Input1", int),
            Field.input("Input2", int),
            Field.input("Input3", int),
            Field.output("Output1", int),
            Field.output("Output2", int),
            Field.output("Output3", int),
        ),
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    # always
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output1),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input1),
        mode=ComputedValueMode.ALWAYS,
    )
    # if source is set
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output2),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input2),
        mode=ComputedValueMode.IF_SOURCE_SET,
    )
    # if target is unset x2 (all but first should be ignored)
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output3),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input3),
        mode=ComputedValueMode.IF_TARGET_UNSET,
    )
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output3),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input2),
        mode=ComputedValueMode.IF_TARGET_UNSET,
    )
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output3),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input1),
        mode=ComputedValueMode.IF_TARGET_UNSET,
    )
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, inputs={"Input1": 1, "Input2": 2, "Input3": 3})
    assert runner.outputs and runner.outputs.Output1 == 1  # Output 1 == Input1 (always)
    assert runner.outputs and runner.outputs.Output2 == 2  # Output2 == Input2?
    assert runner.outputs and runner.outputs.Output3 == 3  # Output3 == Input3 (first set)


@simulated_runtime()
async def test_run_flow_create_in_test_mode(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Flow with Start->Complete in test mode, creating a simple Node. Should be in same node."""
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
Flow1.parent.append(block)
return {'Block': block}
"""),
        fields=(Field.output("Block", Block, is_required=True),),
    )
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Block),
        source=(Create, PathElementType.RUN, Run.get_property("outputs"), Create.fields.Block),
    )
    Flow1.actions.extend(Start, Create, Complete)
    Start.connect(LinkType.REQUIRE, Create, is_manual=True)
    Create.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, mode=NodeMode.TEST)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 5
    assert all(r.mode == NodeMode.TEST for r in runner.tracked_run.runs)
    assert runner.outputs and isinstance(runner.outputs.Block, Block)
    assert runner.outputs.Block.mode == NodeMode.TEST


@simulated_runtime()
async def test_run_flow_computed_run_options(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Run a Flow with computed run options."""
    Flow1 = Flow.new("Flow1", fields=(Field.input("Attempts", int),))
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("""
if len(run.attempts) < 6:
    raise RetryableError("not enough attempts")
else:
    pass  # yay!
"""),
    )
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Code1, Complete)
    Start.connect(LinkType.REQUIRE, Code1, is_manual=True)
    Code1.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    Code1.set_computed(
        target=(
            PathElementType.RUN,
            Run.get_property("options"),
            RunOptions.get_property("max_attempts"),
        ),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Attempts),
    )
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, inputs={"Attempts": 6})
    assert runner.status == RunStatus.COMPLETED


@simulated_runtime()
async def test_run_flow_link_from_nowhere(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a flow with a link from nowhere. Should not be run and just be ignored."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Nowhere = Action.new(ActionType.START, "Nowhere")  # not added to flow/graph
    Nowhere.connect(LinkType.REQUIRE, Complete, parent=Flow1)
    Start.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


@simulated_runtime()
async def test_run_flow_link_to_nowhere(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a flow with a link to nowhere. Should not be run and just be ignored."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Nowhere = Action.new(ActionType.START, "Nowhere")  # not added to flow/graph
    Flow1.actions.extend(Start, Nowhere, Complete)
    Start.connect(LinkType.REQUIRE, Nowhere, parent=Flow1, is_manual=True)
    Start.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    Nowhere.delete()
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 4


@simulated_runtime()
async def test_run_flow_force_invalid_output(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Complete the flow with invalid output (should fail)."""
    Flow1 = Flow.new(
        "Flow1",
        fields=(Field.output("Output1", str, is_required=True),),
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
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
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Code1, Complete)
    Start.connect(LinkType.REQUIRE, Code1, is_manual=True)
    Code1.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE


@simulated_runtime()
async def test_run_flow_error(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a code Action that raises an error. Flow should abort and fail."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("raise ValueError"))
    Flow1.actions.extend(Start, Code1)
    Start.connect(LinkType.REQUIRE, Code1, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


@simulated_runtime()
async def test_run_flow_abort(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a long async flow script and abort it. All pending actions should be aborted."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("await asyncio.sleep(5)"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Code1, Complete)
    Start.connect(LinkType.REQUIRE, Code1, is_manual=True)
    Code1.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    run = create_run_from_node(Flow1, isolate=True)
    run_task = asyncio.create_task(runtime.run_in_runtime(run, return_error=True))
    # kill after 0.5s
    await asyncio.sleep(0.5)
    runtime.runtime.stop_run(run)
    runner = await run_task
    # flow should be aborted
    assert runner.status == RunStatus.ABORTED
    assert runner.tracked_run
    assert runner.tracked_run.duration and runner.tracked_run.duration.total_seconds() < 1
    # inner code action should also be aborted
    assert runner.runners[2].node == Code1 and runner.runners[2].status == RunStatus.ABORTED
