from bench.language.block import Block
from bench.language.const import BlockType, EditOperationType
from bench.language.field import Field, to_type
from bench.language.run import RunErrorType, RunOptions
from bench.test.unit.conftest import RuntimeHandle


async def test_edit_nested_objects(hosted_runtime: RuntimeHandle):
    """Edits to nested objects should be traced correctly"""
    session = hosted_runtime.session
    Class1 = Block.new(
        BlockType.CLASS,
        name="Class1",
        fields=[Field.member("Integer", int), Field.member("String", str)],
    )
    Class1.fields.append(Field.member("Class1", Class1))
    Value1 = Block.new(BlockType.VALUE, name="Value1")
    hosted_runtime.page().blocks.extend(Class1, Value1)
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
    Value1.text = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [Block.get_property("text").key]

    # nested scalar struct set
    Value1.run_options = RunOptions(max_attempts=3)
    Value1.run_options.max_attempts = 4
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        Block.get_property("run_options").key,
        RunOptions.get_property("max_attempts").key,
    ]

    # nested scalar struct clear
    Value1.run_options.max_concurrency = None
    assert get_last_operation().type == EditOperationType.CLEAR
    assert get_last_operation().path == [
        Block.get_property("run_options").key,
        RunOptions.get_property("max_concurrency").key,
    ]

    # nested scalar parent_key set
    Value1.value_type = to_type(Class1)
    Value1.value = Class1(Integer=1, String="two")
    Value1.value.Integer = 2
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        Block.get_property("value_packed").key,
        Class1.fields.Integer.key,
    ]

    # root list set
    Value1.roles = [Class1]
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [Block.get_property("roles").key]

    # root list parent_key clear
    Value1.roles = []
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [Block.get_property("roles").key]

    # nested list parent_key set
    Value1.run_options.retry_on = [RunErrorType.CODE_INVALID, RunErrorType.INVALID_VALUE]
    assert get_last_operation().type == EditOperationType.SET
    assert get_last_operation().path == [
        Block.get_property("run_options").key,
        RunOptions.get_property("retry_on").key,
    ]

    # commit
    await session.commit()
