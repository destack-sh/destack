from typing import cast

from bench.language import (
    Class,
    EditOperationType,
    Field,
    Node,
    View,
    ViewType,
)
from bench.language.core import text
from bench.language.view.view import Offset, ThreadView
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_trace_edits(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Edits to nested objects should be traced correctly."""
    session = runtime.session
    Message1 = Class.new("Message1", Field.member("Integer", int), Field.member("String", str))
    Message1.fields.append(Field.member("Message1", Message1))
    View1 = cast(ThreadView, View.new(ViewType.THREAD, "ThreadView1"))
    Value1 = Class.new("Value1", Field.member("Value", str))
    runtime.page().extend(Message1, View1, Value1)
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
    View1.position = Offset(left=3, top=4)
    View1.position.left = 4
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        View1.get_property("position").key,
        Offset.get_property("left").key,
    ]

    # nested scalar struct clear
    View1.position.left = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [
        View1.get_property("position").key,
        Offset.get_property("left").key,
    ]

    # subtype set
    View1.draft_text = text("Hello, world!")
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        Node.get_property("subnode_packed").key,
        str(ViewType.THREAD),
        ThreadView.get_property("draft_text").key,
    ]

    # subtype clear (indirect via computed property)
    View1.draft_text = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [
        Node.get_property("subnode_packed").key,
        str(ViewType.THREAD),
        ThreadView.get_property("draft_text").key,
    ]

    # commit
    await session.commit()
