from bench.language import (
    Action,
    ActionType,
    Class,
    EditOperationType,
    Field,
    Flow,
    Node,
    Vector2,
)
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_trace_edits(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Edits to nested objects should be traced correctly."""
    session = runtime.session
    Message1 = Class.new("Message1", Field.member("Integer", int), Field.member("String", str))
    Message1.fields.append(Field.member("Message1", Message1))
    Flow1 = Flow.new("Flow1")
    Action1 = Action.new(DuplicateAction, name="Text1")
    Flow1.actions.append(Action1)
    Value1 = Class.new("Value1", Field.member("Value", str))
    runtime.page().extend(Message1, Flow1, Value1)
    await runtime.commit()

    def get_last_operation():
        last_edit = session.tx._pending_edit_events[-1]
        assert last_edit.operations, f"no operations for last edit: {last_edit!r}"
        return last_edit.operations[-1]

    # root scalar parent_key set
    Value1.name = "Value2"
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [Class.get_property("name").key]

    # root scalar parent_key clear
    Value1.icon = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [Class.get_property("icon").key]

    # nested scalar struct set
    Action1.position = Vector2(x=3.0, y=4.0)
    Action1.position.x = 4.0
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        Action.get_property("position").key,
        Vector2.get_property("x").key,
    ]

    # nested scalar struct clear
    Action1.position.x = 5.0
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        Action.get_property("position").key,
        Vector2.get_property("x").key,
    ]

    # subtype set
    Action1.is_shallow = True
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        Node.get_property("subnode_packed").key,
        str(ActionType.DUPLICATE),
        DuplicateAction.get_property("is_shallow").key,
    ]

    # subtype clear (indirect via computed property)
    Action1.is_shallow = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [
        Node.get_property("subnode_packed").key,
        str(ActionType.DUPLICATE),
        DuplicateAction.get_property("is_shallow").key,
    ]

    # commit
    await session.commit()
