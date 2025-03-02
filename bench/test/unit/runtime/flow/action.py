from bench.language import (
    Action,
    ActionType,
    ComputedValueMode,
    Database,
    Field,
    Flow,
    LinkType,
    PathElementType,
    Record,
    Run,
    RunStatus,
    Text,
)
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_flow_fail_action(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a Flow with a Fail action."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Fail = Action.new(
        ActionType.FAIL, "Fail", error_title="Fail title", error_text=Text.plain("Fail text")
    )
    Flow1.actions.extend(Start, Fail)
    Start.connect(LinkType.SELECT, Fail)
    runtime.page().append(Flow1)
    await runtime.commit()

    # flow
    runner = await runtime.run_in_runtime(Flow1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error
    assert runner.error.title == "Fail title"
    assert runner.error.text == Text.plain("Fail text")

    # run directly with custom inputs
    runner = await runtime.run_in_runtime(
        Fail,
        inputs={"error_title": "Custom title", "error_text": Text.plain("Custom text")},
        return_error=True,
    )
    assert runner.status == RunStatus.FAILED
    assert runner.error
    assert runner.error.title == "Custom title"
    assert runner.error.text == Text.plain("Custom text")


@simulated_runtime()
async def test_run_flow_create_action(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a CreateAction to create a Record."""
    Database1 = Database.new("Database1", Field.member("Rating", int))
    Flow1 = Flow.new("Flow1")
    Create = Action.new(
        ActionType.CREATE,
        "Create",
        node_partial=Record.partial(database=Database1, Rating=2),
    )
    Flow1.actions.append(Create)
    runtime.page().append(Database1)
    runtime.page().append(Flow1)
    await runtime.commit()

    # run from action
    runner = await runtime.run_in_runtime(Create)
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=2)
    assert record is not None

    # run from action inputs
    runner = await runtime.run_in_runtime(
        Create, inputs={"node_partial": Record.partial(database=Database1, Rating=3)}
    )
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=3)
    assert record is not None


@simulated_runtime()
async def test_run_flow_create_action_dynamic(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Run a CreateAction with a dynamic node_partial."""
    Database1 = Database.new(
        "Database1",
        Field.member("Rating", int),
    )
    Flow1 = Flow.new(
        "Flow1",
        fields=(Field.input("Name", str), Field.input("Rating", int)),
    )
    Start = Action.new(ActionType.START, "Start")
    Create = Action.new(
        ActionType.CREATE,
        "Create",
        node_partial=Record.partial(database=Database1, name="My Custom Record", Rating=1),
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
    Start.connect(LinkType.SELECT, Create)
    runtime.page().append(Database1)
    runtime.page().append(Flow1)
    await runtime.commit()

    # run from flow (with partial override)
    runner = await runtime.run_in_runtime(Flow1, inputs={"Rating": 2})
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=2)
    assert record is not None
    assert record.name == "My Custom Record"

    # run from flow (with full override)
    runner = await runtime.run_in_runtime(Flow1, inputs={"Name": "My Other Record", "Rating": 3})
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=3)
    assert record is not None
    assert record.name == "My Other Record"


@simulated_runtime()
async def test_run_flow_duplicate_action(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a DuplicateAction to clone a Record."""
    Database1 = Database.new("Database1", Field.member("Rating", int))
    Record1 = Database1.records.create(Rating=1)
    Flow1 = Flow.new("Flow1")
    Clone = Action.new(ActionType.DUPLICATE, "Clone")
    Flow1.actions.append(Clone)
    runtime.page().append(Database1)
    runtime.page().append(Flow1)
    await runtime.commit()

    Record1._detach_rec()  # detach to also test remote loading
    runner = await runtime.run_in_runtime(
        Clone,
        inputs={"node": Record1, "node_partial": Record.partial(database=Database1, Rating=3)},
    )
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=3)
    assert record is not None


@simulated_runtime()
async def test_run_flow_update_action(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run an UpdateAction to update a Record."""
    Database1 = Database.new("Database1", Field.member("Rating", int))
    Record1 = Database1.records.create(name="Record1", Rating=1)
    Flow1 = Flow.new("Flow1")
    Update = Action.new(ActionType.UPDATE, "Update")
    Flow1.actions.append(Update)
    runtime.page().append(Database1)
    runtime.page().append(Flow1)
    await runtime.commit()

    Record1._detach_rec()  # detach to also test remote loading
    runner = await runtime.run_in_runtime(
        Update,
        inputs={
            "node": Record1,
            "node_partial": Record.partial(database=Database1, name="Record1.1", Rating=3),
        },
    )
    assert runner.status == RunStatus.COMPLETED
    record = await Database1.records.get(Rating=3)
    assert record is not None
    assert record.name == "Record1.1"


@simulated_runtime()
async def test_run_flow_delete_action(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a DeleteAction to delete a Record."""
    Database1 = Database.new("Database1", Field.member("Rating", int))
    Record1 = Database1.records.create(Rating=1)
    Flow1 = Flow.new("Flow1")
    Delete = Action.new(ActionType.DELETE, "Delete")
    Flow1.actions.append(Delete)
    runtime.page().extend(Database1, Flow1)
    await runtime.commit()

    Record1._detach_rec()  # detach to also test remote loading
    runner = await runtime.run_in_runtime(Delete, inputs={"node": Record1})
    assert runner.status == RunStatus.COMPLETED
    assert await Database1.records.search() == []
