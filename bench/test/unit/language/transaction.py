from bench.language.action import Action, ActionType, DuplicateAction
from bench.language.block import Block, FlowBlock, VariableBlock
from bench.language.const import BlockType, EditOperationType
from bench.language.field import Field
from bench.language.node import Node
from bench.language.view import Vector2
from bench.test.unit.conftest import RuntimeHandle


async def test_trace_edits(hosted_runtime: RuntimeHandle):
    """Edits to nested objects should be traced correctly."""
    session = hosted_runtime.session
    Message1 = Block.new(
        BlockType.MESSAGE,
        name="Message1",
        fields=[Field.member("Integer", int), Field.member("String", str)],
    )
    Message1.fields.append(Field.member("Message1", Message1))
    Flow1 = Block.new(FlowBlock, name="Flow1")
    Action1 = Action.new(DuplicateAction, name="Text1")
    Flow1.actions.append(Action1)
    Value1 = Block.new(VariableBlock, name="Value1")
    hosted_runtime.page().blocks.extend(Message1, Flow1, Value1)
    await session.commit()

    def get_last_operation():
        last_edit = session.tx._pending_edit_events[-1]
        assert last_edit.operations, f"no operations for last edit: {last_edit!r}"
        return last_edit.operations[-1]

    # root scalar parent_key set
    Value1.name = "Value2"
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [Block.get_property("name").key]

    # root scalar parent_key clear
    Value1.icon = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [Block.get_property("icon").key]

    # root list set
    Value1.roles = [Message1]
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [Block.get_property("roles").key]

    # root list parent_key clear
    Value1.roles = []
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [Block.get_property("roles").key]

    # nested scalar struct set
    Action1.position = Vector2(x=3, y=4)
    Action1.position.x = 4
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        Action.get_property("position").key,
        Vector2.get_property("x").key,
    ]

    # nested scalar struct clear
    Action1.position.x = 5
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
        Action.get_property("is_shallow").key,
    ]

    # subtype clear (indirect via computed property)
    Action1.is_shallow = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [
        Node.get_property("subnode_packed").key,
        str(ActionType.DUPLICATE),
        Action.get_property("is_shallow").key,
    ]

    # commit
    await session.commit()
