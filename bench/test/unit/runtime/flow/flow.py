import asyncio

from bench.language import (
    Action,
    ActionType,
    Block,
    BlockType,
    Breakpoint,
    BreakpointScope,
    Code,
    ComputedValueMode,
    ErrorType,
    Field,
    Interruption,
    InterruptionStatus,
    NodeMode,
    PathElementType,
    PipeType,
    Record,
    Run,
    RunOptions,
    RunStatus,
    Text,
    code,
)
from bench.runtime import Interrupted, create_run_from_node, make_runner
from bench.runtime.flow.action import CodeActionRunner
from bench.test.unit.conftest import RuntimeHandle


async def test_run_flow_empty(hosted_runtime: RuntimeHandle):
    """Empty Code without any fields should fail."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1)
    assert runner.tracked_run and not runner.tracked_run.runs  # no nested runs


async def test_run_flow_spurious(hosted_runtime: RuntimeHandle):
    """Flow with Actions that go nowhere."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Flow1.actions.append(Action.new(ActionType.START, "Start"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    # don't actually connect the actions
    Flow1.actions.extend(Start, Complete, Code1)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 1  # just Start


async def test_run_flow_trivial(hosted_runtime: RuntimeHandle):
    """Trivial flow with Start->Complete, no value."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_with_default_values(hosted_runtime: RuntimeHandle):
    """Run a Flow with default values in Complete action."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(
            Field.output("Output1", int, is_required=True),
            Field.output("Output2", bool),
            Field.output("Output3", str, is_required=True),
        ),
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(
        ActionType.COMPLETE, "Complete", parent=Flow1, inputs={"Output1": 1, "Output3": "MyString"}
    )
    Flow1.actions.extend(Start, Complete)
    Start.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_code(hosted_runtime: RuntimeHandle):
    """Run a code action with values."""
    Flow1 = Block.new(
        BlockType.FLOW,
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
    Start.connect(PipeType.CALL, Code1)
    Code1.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, inputs={"Input1": 2})
    assert runner.tracked_run and len(runner.tracked_run.runs) == 5
    assert runner.outputs and runner.outputs.Output1 == 4


async def test_run_flow_computed_value_chain(hosted_runtime: RuntimeHandle):
    """Run a Flow with Actions chaining computed inputs."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(
            Field.variable("BoolIn", bool),
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
        prev.connect(PipeType.CALL, Code)
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
    prev.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, variables={"BoolIn": True}, inputs={"IntIn": 0})
    assert runner.outputs and runner.outputs.BoolOut is False
    assert runner.outputs and runner.outputs.IntOut == 4


async def test_run_flow_invalid_computed_source(hosted_runtime: RuntimeHandle):
    """Run a Flow with invalid computed values (invalid source). Should fail."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(Field.input("Input", int), Field.output("Output", int)),
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(PipeType.CALL, Complete)
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output),
        # missing PathElementType.RUN for source, and Block has no inputs
        source=(Flow1, Run.get_property("inputs"), Flow1.fields.Input),
    )
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_COMPUTED


async def test_run_flow_invalid_computed_target(hosted_runtime: RuntimeHandle):
    """Run a Flow with an invalid computed value (invalid target). Should pass (?)."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(Field.input("Input", int), Field.output("Output", int)),
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(PipeType.CALL, Complete)
    Complete.set_computed(
        # refers to Output, but we delete output below (oh no!, should be ignored)
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input),
    )
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    # delete output Field
    Flow1.fields.Output.delete()
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, inputs={"Input": 1})
    assert runner.status == RunStatus.COMPLETED


async def test_run_flow_computed_value_mode(hosted_runtime: RuntimeHandle):
    """Run a Flow with computed value set if source is set."""
    Flow1 = Block.new(
        BlockType.FLOW,
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
    Start.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
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
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, inputs={"Input1": 1, "Input2": 2, "Input3": 3})
    assert runner.outputs and runner.outputs.Output1 == 1  # Output 1 == Input1 (always)
    assert runner.outputs and runner.outputs.Output2 == 2  # Output2 == Input2?
    assert runner.outputs and runner.outputs.Output3 == 3  # Output3 == Input3 (first set)


async def test_run_flow_code_dynamic(hosted_runtime: RuntimeHandle):
    """Run a Code action with Action.code set dynamically in a Variable."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.variable("Code", Code), Field.output("Output", int))
    )
    Start = Action.new(ActionType.START, "Start")
    Flow1.actions.append(Start)
    Code1 = Action.new(
        ActionType.CODE,
        "Code",
        code=code("return {'Output': 1}"),
        fields=(Field.output("Output", int),),
    )
    Flow1.actions.append(Code1)
    Start.connect(PipeType.CALL, Code1)
    Code1.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Code1.get_property("code")),
        source=(Flow1, PathElementType.RUN, Run.get_property("variables"), Flow1.fields.Code),
    )
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output),
        source=(Code1, PathElementType.RUN, Run.get_property("outputs"), Code1.fields.Output),
    )
    Flow1.actions.append(Complete)
    Code1.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    # override the code to return 2
    runner = await hosted_runtime.run(Flow1, variables={"Code": code("return {'Output': 2}")})
    assert runner.outputs and runner.outputs.Output == 2


async def test_run_flow_create_in_test_mode(hosted_runtime: RuntimeHandle):
    """Flow with Start->Complete in test mode, creating a simple Node. Should be in same node."""
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(Field.output("Block", Block, is_required=True),),
    )
    Start = Action.new(ActionType.START, "Start")
    Create = Action.new(
        ActionType.CODE,
        "Create",
        code=code("""\
block = Block.new(BlockType.TEXT, "Test")
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
    Start.connect(PipeType.CALL, Create)
    Create.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, mode=NodeMode.TEST)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 5
    assert all(r.mode == NodeMode.TEST for r in runner.tracked_run.runs)
    assert runner.outputs and isinstance(runner.outputs.Block, Block)
    assert runner.outputs.Block.mode == NodeMode.TEST


async def test_run_flow_computed_run_options(hosted_runtime: RuntimeHandle):
    """Run a Flow with computed run options."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1", fields=(Field.variable("Attempts", int),))
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
    Start.connect(PipeType.CALL, Code1)
    Code1.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    Code1.set_computed(
        target=(
            PathElementType.RUN,
            Run.get_property("options"),
            RunOptions.get_property("max_attempts"),
        ),
        source=(Flow1, PathElementType.RUN, Run.get_property("variables"), Flow1.fields.Attempts),
    )
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, variables={"Attempts": 6})
    assert runner.status == RunStatus.COMPLETED


async def test_run_flow_pipe_from_nowhere(hosted_runtime: RuntimeHandle):
    """Run a flow with a pipe from nowhere. Should not be run and just be ignored."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Nowhere = Action.new(ActionType.START, "Nowhere")  # not added to flow/graph
    Nowhere.connect(PipeType.CALL, Complete, parent=Flow1)
    Start.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_pipe_to_nowhere(hosted_runtime: RuntimeHandle):
    """Run a flow with a pipe to nowhere. Should not be run and just be ignored."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Nowhere = Action.new(ActionType.START, "Nowhere")  # not added to flow/graph
    Flow1.actions.extend(Start, Complete)
    Start.connect(PipeType.CALL, Nowhere, parent=Flow1)
    Start.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) == 5


async def test_run_flow_force_invalid_output(hosted_runtime: RuntimeHandle):
    """Complete the flow with invalid output (should fail)."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.output("Output1", str, is_required=True),)
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Start.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE


async def test_run_flow_force_invalid_input(hosted_runtime: RuntimeHandle):
    """Run a action with a trigger port that forces a Run of a Action with invalid inputs (should fail)."""
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.output("Output1", str, is_required=True),)
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
    Start.connect(PipeType.CALL, Code1)
    Code1.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE


async def test_run_flow_error(hosted_runtime: RuntimeHandle):
    """Run a code Action that raises an error. Flow should abort and fail."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("raise ValueError"))
    Flow1.actions.extend(Start, Code1)
    Start.connect(PipeType.CALL, Code1)
    hosted_runtime.page().blocks.extend(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_error_with_error_suppressed(hosted_runtime: RuntimeHandle):
    """Run a code action with an error with failure suppressed. Flow should complete."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("raise ValueError('error')"),
        run_options=RunOptions(suppress_fail=True),
    )
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Code1, Complete)
    Start.connect(PipeType.CALL, Code1)
    Code1.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.extend(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1)
    assert runner.status == RunStatus.COMPLETED
    assert runner.tracked_run and len(runner.tracked_run.runs) == 3


async def test_run_flow_fail_action(hosted_runtime: RuntimeHandle):
    """Run a Flow with a Fail action."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Fail = Action.new(
        ActionType.FAIL, "Fail", error_title="Fail title", error_text=Text.plain("Fail text")
    )
    Flow1.actions.extend(Start, Fail)
    Start.connect(PipeType.CALL, Fail)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    # flow
    runner = await hosted_runtime.run(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error
    assert runner.error.title == "Fail title"
    assert runner.error.text == Text.plain("Fail text")

    # run directly with custom inputs
    runner = await hosted_runtime.run(
        Fail,
        inputs={"error_title": "Custom title", "error_text": Text.plain("Custom text")},
        return_error=True,
    )
    assert runner.status == RunStatus.FAILED
    assert runner.error
    assert runner.error.title == "Custom title"
    assert runner.error.text == Text.plain("Custom text")


async def test_run_flow_create_action(hosted_runtime: RuntimeHandle):
    """Run a CreateAction to create a Record."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=(Field.member("Rating", int),))
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Create = Action.new(
        ActionType.CREATE,
        "Create",
        node_partial=Record.partial(block=Database1, Rating=2),
    )
    Flow1.actions.append(Create)
    hosted_runtime.page().blocks.extend(Database1, Flow1)
    await hosted_runtime.commit()

    # run from action
    runner = await hosted_runtime.run(Create)
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=2)
    assert record is not None

    # run from action inputs
    runner = await hosted_runtime.run(
        Create, inputs={"node_partial": Record.partial(block=Database1, Rating=3)}
    )
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=3)
    assert record is not None


async def test_run_flow_create_action_dynamic(hosted_runtime: RuntimeHandle):
    """Run a CreateAction with a dynamic node_partial."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=(Field.member("Rating", int),))
    Flow1 = Block.new(
        BlockType.FLOW, "Flow1", fields=(Field.input("Name", str), Field.input("Rating", int))
    )
    Start = Action.new(ActionType.START, "Start")
    Create = Action.new(
        ActionType.CREATE,
        "Create",
        node_partial=Record.partial(block=Database1, name="My Custom Record", Rating=1),
    )
    Create.set_computed(
        target=(
            PathElementType.RUN,
            Run.get_property("inputs"),
            Create.get_property("node_partial"),
            Record.get_property("name"),
        ),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Name),
        mode=ComputedValueMode.IF_SOURCE_SET,
    )
    Create.set_computed(
        target=(
            PathElementType.RUN,
            Run.get_property("inputs"),
            Create.get_property("node_partial"),
            Database1.fields.Rating,
        ),
        source=(Flow1, PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Rating),
        mode=ComputedValueMode.IF_SOURCE_SET,
    )
    Flow1.actions.extend(Start, Create)
    Start.connect(PipeType.CALL, Create)
    hosted_runtime.page().blocks.extend(Database1, Flow1)
    await hosted_runtime.commit()

    # run from flow (with partial override)
    runner = await hosted_runtime.run(Flow1, inputs={"Rating": 2})
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=2)
    assert record is not None
    assert record.name == "My Custom Record"

    # run from flow (with full override)
    runner = await hosted_runtime.run(Flow1, inputs={"Name": "My Other Record", "Rating": 3})
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=3)
    assert record is not None
    assert record.name == "My Other Record"


async def test_run_flow_clone_action(hosted_runtime: RuntimeHandle):
    """Run a CloneAction to clone a Record."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=(Field.member("Rating", int),))
    Record1 = Database1.records.create(Rating=1)
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Clone = Action.new(ActionType.DUPLICATE, "Clone")
    Flow1.actions.append(Clone)
    hosted_runtime.page().blocks.extend(Database1, Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(
        Clone, inputs={"node": Record1, "node_partial": Record.partial(block=Database1, Rating=3)}
    )
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=3)
    assert record is not None


async def test_run_flow_update_action(hosted_runtime: RuntimeHandle):
    """Run an UpdateAction to update a Record."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=(Field.member("Rating", int),))
    Record1 = Database1.records.create(name="Record1", Rating=1)
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Update = Action.new(ActionType.UPDATE, "Update")
    Flow1.actions.append(Update)
    hosted_runtime.page().blocks.extend(Database1, Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(
        Update,
        inputs={
            "node": Record1,
            "node_partial": Record.partial(block=Database1, name="Record1.1", Rating=3),
        },
    )
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=3)
    assert record is not None
    assert record.name == "Record1.1"


async def test_run_flow_delete_action(hosted_runtime: RuntimeHandle):
    """Run a DeleteAction to delete a Record."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=(Field.member("Rating", int),))
    Record1 = Database1.records.create(Rating=1)
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Delete = Action.new(ActionType.DELETE, "Delete")
    Flow1.actions.append(Delete)
    hosted_runtime.page().blocks.extend(Database1, Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Delete, inputs={"node": Record1})
    assert runner.status == RunStatus.COMPLETED
    assert await Database1.records.search() == []


async def test_run_flow_race(hosted_runtime: RuntimeHandle):
    """Run multiple actions in parallel, losers should be aborted on completion of winner."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Race1 = Action.new(ActionType.CODE, "Race1", code=code("await asyncio.sleep(1)"))
    Race2 = Action.new(ActionType.CODE, "Race2", code=code("await asyncio.sleep(2)"))
    Race3 = Action.new(ActionType.CODE, "Race3", code=code("await asyncio.sleep(3)"))
    Flow1.actions.extend(Start, Race1, Race2, Race3, Complete)
    Start.connect(PipeType.CALL, Race1)
    Start.connect(PipeType.CALL, Race2)
    Start.connect(PipeType.CALL, Race3)
    Race1.connect(PipeType.CALL, Complete)
    Race2.connect(PipeType.CALL, Complete)
    Race3.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow1)
    assert runner.tracked_run
    aborted_runs = [r for r in runner.tracked_run.runs if r.status == RunStatus.ABORTED]
    assert len(aborted_runs) == 2  # the two losers should be aborted
    assert len(runner.tracked_run.runs) == 9  # all actions & pipes should run exactly once


async def test_run_flow_infinite_loop(local_runtime: RuntimeHandle):
    """Runs an infinite loop that's not infinite because it also completes immediately."""
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Loop = Action.new(ActionType.CODE, "Loop", code=code("pass"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Loop, Complete)
    Start.connect(PipeType.CALL, Loop)
    Loop.connect(PipeType.CALL, Loop)  # infinite!
    Loop.connect(PipeType.CALL, Complete)
    local_runtime.page().blocks.append(Flow1)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow1)
    assert runner.tracked_run and len(runner.tracked_run.runs) <= 10  # should complete quickly


async def test_run_flow_call_none(hosted_runtime: RuntimeHandle):
    """Runs a Flow with no calls selected."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Route = Action.new(ActionType.CODE, "Router", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow.actions.extend(Start, Route, Code2, Code3, Complete)
    Start.connect(PipeType.CALL, Route)
    Route.connect(PipeType.SELECT, Complete)
    Route.connect(PipeType.SELECT, Code2)
    Route.connect(PipeType.SELECT, Code3)
    hosted_runtime.page().blocks.append(Flow)
    await hosted_runtime.commit()

    Route.code = code("pass")
    runner = await hosted_runtime.run(Flow)
    assert runner.tracked_run
    assert not runner.tracked_run.has(Code2, Code3, Complete)


async def test_run_flow_call_route(hosted_runtime: RuntimeHandle):
    """Runs a Flow with forward calls selected."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Route = Action.new(ActionType.CODE, "Router", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    Code4 = Action.new(ActionType.CODE, "Code4", code=code("pass"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow.actions.extend(Start, Route, Code2, Code3, Code4, Complete)
    Start.connect(PipeType.CALL, Route)
    Route.connect(PipeType.SELECT, Complete)
    Route.connect(PipeType.SELECT, Code2)
    Route.connect(PipeType.SELECT, Code3)
    Route.connect(PipeType.SELECT, Code4)
    hosted_runtime.page().blocks.append(Flow)
    await hosted_runtime.commit()

    # Route: Code2 + Code3
    Route.code = code("""\
return {
    "plans": [call_serial(call(Code2), call(Code3))],
}
""")
    runner = await hosted_runtime.run(Flow)
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
    runner = await hosted_runtime.run(Flow)
    assert runner.tracked_run
    assert runner.tracked_run.has(Code4)
    assert not runner.tracked_run.has(Complete)
    assert not runner.tracked_run.has(Code2)
    assert not runner.tracked_run.has(Code3)

    # Route: Complete + Code2
    Route.code = code("""\
return {
    "plans": [call_serial(call(Complete), call(Code2))],
}
""")
    runner = await hosted_runtime.run(Flow)
    assert runner.tracked_run
    assert runner.tracked_run.has(Complete)
    assert runner.tracked_run.has(Code2)
    assert not runner.tracked_run.has(Code3)
    assert not runner.tracked_run.has(Code4)


async def test_run_flow_call_tool(hosted_runtime: RuntimeHandle):
    """Run a Flow with a tool call."""
    Flow = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=(
            Field.input("Input1", str),
            Field.output("Output1", str),
        ),
    )
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    Tool1 = Action.new(ActionType.TOOL, "Tool1")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow.actions.extend(Start, Code1, Tool1, Complete)
    Start.connect(PipeType.CALL, Code1)
    Code1.connect(PipeType.CALL, Tool1)
    Tool1.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow)
    await hosted_runtime.commit()

    # run tool action directly
    runner = await hosted_runtime.run(
        Tool1,
        inputs={"type": ActionType.CODE, "code": code("pass")},
    )
    assert len(runner.runners) == 1
    assert isinstance(runner.runners[0], CodeActionRunner)
    assert runner.runners[0].inputs.code == code("pass")

    # running flow as is should fail (at tool, because tool is unset)
    runner = await hosted_runtime.run(Flow, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.RUN_IMPOSSIBLE

    # run tool within flow via calls
    Code1.code = code("""\
return {
    "plans": [call(Tool1, type=ActionType.CODE, code=code("pass"))],
}
""")
    runner = await hosted_runtime.run(Flow)
    assert runner.tracked_run


async def test_run_flow_call_plan(hosted_runtime: RuntimeHandle):
    """Run a Flow with a CallPlan."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    Code2 = Action.new(ActionType.CODE, "Code2", code=code("pass"))
    Code3 = Action.new(ActionType.CODE, "Code3", code=code("pass"))
    Code4 = Action.new(ActionType.CODE, "Code4", code=code("pass"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow.actions.extend(Start, Code1, Code2, Code3, Code4, Complete)
    Start.connect(PipeType.CALL, Code1)
    Code1.connect(PipeType.CALL, Code2)
    Code1.connect(PipeType.SELECT, Code3)
    Code1.connect(PipeType.SELECT, Code4)
    Code1.connect(PipeType.SELECT, Complete)
    await hosted_runtime.commit()

    # run flow with call plan
    runner = await hosted_runtime.run(Flow)
    assert runner.tracked_run

    # nocheckin


async def test_run_flow_abort(hosted_runtime: RuntimeHandle):
    """Run a long async flow script and abort it. All pending actions should be aborted."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("await asyncio.sleep(5)"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow.actions.extend(Start, Code1, Complete)
    Start.connect(PipeType.CALL, Code1)
    Code1.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow)
    await hosted_runtime.commit()

    run = create_run_from_node(Flow)
    run_task = asyncio.create_task(hosted_runtime.run(run, return_error=True))
    # kill after 0.5s
    await asyncio.sleep(0.5)
    hosted_runtime.runtime.stop(run)
    runner = await run_task
    # flow should be aborted
    assert runner.status == RunStatus.ABORTED
    assert runner.tracked_run
    assert runner.tracked_run.duration and runner.tracked_run.duration.total_seconds() < 1
    # inner code action should also be aborted
    assert runner.runners[2].node == Code1 and runner.runners[2].status == RunStatus.ABORTED


async def test_run_flow_yield(hosted_runtime: RuntimeHandle):
    """Run a Flow with a Yield action, then resume from the Yield."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Yield = Action.new(ActionType.YIELD, "Yield")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow.actions.extend(Start, Yield, Complete)
    Start.connect(PipeType.CALL, Yield)
    Yield.connect(PipeType.CALL, Complete)
    hosted_runtime.page().blocks.append(Flow)
    await hosted_runtime.commit()

    # run up to yield
    runner = await hosted_runtime.run(Flow)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interruption
    assert not runner.tracked_run.terminated_at
    assert len(runner.attempts) == 1

    # resume run (without handling Interruption)
    runner = await hosted_runtime.run(runner.tracked_run)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interruption
    assert not runner.tracked_run.terminated_at
    assert len(runner.attempts) == 1  # should be the same attempt

    # handle interruption
    runner.tracked_run.interruption.complete()

    # resume run (after handling Interruption)
    runner = await hosted_runtime.run(runner.tracked_run)
    assert runner.status == RunStatus.COMPLETED
    assert runner.tracked_run
    assert len(runner.attempts) == 1


async def test_run_flow_yield_nested(local_runtime: RuntimeHandle):
    """Run a FLow inside another Flow and yield from there. Should propagate and resume properly."""
    # inner flow
    FlowInner = Block.new(BlockType.FLOW, "FlowInner")
    StartInner = Action.new(ActionType.START, "StartInner")
    YieldInner = Action.new(ActionType.YIELD, "YieldInner")
    CompleteInner = Action.new(ActionType.COMPLETE, "CompleteInner")
    FlowInner.actions.extend(StartInner, YieldInner, CompleteInner)
    StartInner.connect(PipeType.CALL, YieldInner)
    YieldInner.connect(PipeType.CALL, CompleteInner)

    # outer flow
    FlowOuter = Block.new(BlockType.FLOW, "FlowOuter")
    StartOuter = Action.new(ActionType.START, "Start")
    ActionOuter = Action.new(ActionType.TOOL, "Action", tool=FlowInner)
    CompleteOuter = Action.new(ActionType.COMPLETE, "Complete")
    FlowOuter.actions.extend(StartOuter, ActionOuter, CompleteOuter)
    StartOuter.connect(PipeType.CALL, ActionOuter)
    ActionOuter.connect(PipeType.CALL, CompleteOuter)

    local_runtime.page().blocks.extend(FlowInner, FlowOuter)
    await local_runtime.commit()

    # run up to yield
    runner = await local_runtime.run(FlowOuter)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run

    # resume run (without handling Interruption)
    runner = await local_runtime.run(runner.tracked_run)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interrupted_at and runner.tracked_run.interruption

    # handle interruption
    runner.tracked_run.interruption.complete()

    # resume run (after handling Interruption)
    runner = await local_runtime.run(runner.tracked_run)
    assert runner.status == RunStatus.COMPLETED


async def test_run_flow_yield_cancelled(local_runtime: RuntimeHandle):
    """Run a Flow with a Yield action, then cancel it."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Yield = Action.new(ActionType.YIELD, "Yield")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow.actions.extend(Start, Yield, Complete)
    Start.connect(PipeType.CALL, Yield)
    Yield.connect(PipeType.CALL, Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow)
    assert runner.status == RunStatus.YIELDED
    assert runner.tracked_run
    assert runner.tracked_run.interruption
    runner.tracked_run.interruption.cancel()
    runner = await local_runtime.run(runner.tracked_run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INTERRUPTION_CANCELLED


async def test_run_flow_breakpoint(local_runtime: RuntimeHandle):
    """Run a Flow with breakpoints all over. Should yield and resume properly."""
    Flow = Block.new(
        BlockType.FLOW,
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
    Flow.actions.extend(Start, Yield, Action1, Complete)
    StartToYield = Start.connect(
        PipeType.CALL,
        Yield,
        run_options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_failed()]),
    )
    Yield.connect(PipeType.CALL, Action1)
    Action1ToComplete = Action1.connect(
        PipeType.CALL,
        Complete,
        run_options=RunOptions(breakpoints=[Breakpoint.before(), Breakpoint.after_completed()]),
    )
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    # check that all yield points are hit in order
    run = create_run_from_node(Flow)
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
        runner = await local_runtime.run(run)
        assert runner.status == RunStatus.YIELDED
        assert runner.interruption and runner.interruption.runnable == yield_point
        assert runner.tracked_run
        run = runner.tracked_run

        # run up to yield again (without handling Interruption)
        runner = await local_runtime.run(run)
        assert runner.status == RunStatus.YIELDED
        assert runner.interruption and runner.interruption.runnable == yield_point
        assert runner.tracked_run
        run = runner.tracked_run

        # handle interruption
        runner.interruption.complete()

    # run up to completion
    runner = await local_runtime.run(run)
    assert runner.status == RunStatus.COMPLETED


async def test_run_flow_pause_resume(local_runtime: RuntimeHandle):
    """Run a long async Flow and pause it, then resume it."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Action1 = Action.new(ActionType.CODE, "Action1", code=code("await sleep(0.2)"))
    Action2 = Action.new(ActionType.CODE, "Action2", code=code("await sleep(0.2)"))
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow.actions.extend(Start, Action1, Action2, Complete)
    Start.connect(PipeType.CALL, Action1)
    Action1.connect(PipeType.CALL, Action2)
    Action2.connect(PipeType.CALL, Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    # run, pause
    runner = make_runner(local_runtime.runtime, Flow, run="track")
    assert runner.tracked_run
    asyncio.get_event_loop().call_later(0.1, runner.tracked_run.pause)
    try:
        _ = await local_runtime.runtime.run_runner(runner)
    except Interrupted:
        assert runner.status == RunStatus.PAUSED

    # resume
    runner.tracked_run.resume()
    _ = await local_runtime.runtime.run_runner(runner)
    assert runner.status == RunStatus.COMPLETED


async def test_run_flow_autoclose_interruptions(local_runtime: RuntimeHandle):
    """Run and complete a Flow with an Interruption active, it should auto-cancel."""
    Flow = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Yield = Action.new(ActionType.YIELD, "Yield")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Pass = Action.new(ActionType.CODE, "Pass", code=code("pass"))
    Flow.actions.extend(Start, Yield, Pass, Complete)
    Start.connect(PipeType.CALL, Yield)
    Start.connect(PipeType.CALL, Pass)
    Pass.connect(PipeType.CALL, Complete)
    local_runtime.page().blocks.append(Flow)
    await local_runtime.commit()

    runner = await local_runtime.run(Flow)
    assert runner.status == RunStatus.COMPLETED
    assert runner.tracked_run

    interruptions = runner.tracked_run._graph.nodes_of_type(Interruption)
    assert len(interruptions) == 1
    assert interruptions[0].status == InterruptionStatus.CANCELLED
