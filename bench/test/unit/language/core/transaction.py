from bench.language import (
    EditOperationType,
    Field,
    Schema,
)
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_trace_edits(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Edits to nested objects should be traced correctly."""
    session = runtime.session
    Message1 = Schema.new("Message1", Field.member("Integer", int), Field.member("String", str))
    Message1.add_child(Field.member("Message1", Message1))
    Value1 = Schema.new("Value1", Field.member("Value", str))
    runtime.page().add_children(Message1, Value1)
    await runtime.commit()

    def get_last_operation():
        last_edit = session.tx._pending_edit_events[-1]
        assert last_edit.operations, f"no operations for last edit: {last_edit!r}"
        return last_edit.operations[-1]

    # root scalar parent_key set
    Value1.name = "Value2"
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [Schema.get_property("name").key]

    # root scalar parent_key clear
    Value1.icon = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [Schema.get_property("icon").key]

    # commit
    await session.commit()
