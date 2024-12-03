from bench.language.action import Agency
from bench.language.block import Block
from bench.language.code import code
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.node import Node
from bench.test.unit.conftest import RuntimeHandle


async def test_detached_node(local_runtime: RuntimeHandle) -> None:
    """Pack/unpack a detached Node (tree)."""
    # action that takes a node, renames it and returns it
    block = Block.new(
        BlockType.ACTION,
        "Action",
        agency=Agency.CODE,
        fields=(Field.input("Input1", Node, is_required=True), Field.output("Output1", Node)),
        code=code("""\
if Input1.is_attached:
    Input1.name += "Attached"
else:
    Input1.name += "Detached"
return Input1
"""),
    )
    local_runtime.page().blocks.append(block)
    await local_runtime.commit()

    # run with attached node (same reference throughout)
    Choice = Block.new(
        BlockType.CHOICE, "Choice", fields=(Field.option("Option1"), Field.option("Option2"))
    )
    local_runtime.page().blocks.append(Choice)
    await local_runtime.commit()
    runner = await local_runtime.run(block, inputs={"Input1": Choice})
    assert runner.inputs and isinstance(runner.inputs.Input1, Block)
    assert runner.outputs and isinstance(runner.outputs.Output1, Block)
    assert runner.inputs.Input1 == Choice
    assert runner.outputs.Output1 == Choice
    assert runner.inputs.Input1.name == "ChoiceAttached"
    assert runner.outputs.Output1.name == "ChoiceAttached"

    # run with detached node
    # nocheckin: detached node?
    Choice = Block.new(
        BlockType.CHOICE, "Choice", fields=(Field.option("Option1"), Field.option("Option2"))
    )
    runner = await local_runtime.run(block, inputs={"Input1": Choice})
    assert runner.inputs and isinstance(runner.inputs.Input1, Block)
    assert runner.outputs and isinstance(runner.outputs.Output1, Block)
    assert runner.inputs.Input1 == Choice  # same identity
    assert runner.outputs.Output1 == Choice  # same identity
    assert runner.inputs.Input1.name == "Choice"  # but different value
    assert runner.outputs.Output1.name == "ChoiceDetached"
