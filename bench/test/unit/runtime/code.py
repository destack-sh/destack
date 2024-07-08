import pytest

from bench.language import Block, BlockType, Field, code
from bench.runtime.runner import RuntimeRunner


async def test_run_code_with_syntax_error(runner: RuntimeRunner, page: Block):
    block = Block.new(BlockType.CODE, "Invalid", code=code("!!invalid!!"))
    page.blocks.append(block)
    with pytest.raises(SyntaxError):
        await runner.run(block)


async def test_run_code_function_coerce_single(runner: RuntimeRunner, page: Block):
    block = Block.new(
        BlockType.CODE,
        "Function1",
        code=code("""\
return Input1 * 4
"""),
    )
    block.fields.extend(Field.input("Input1", int), Field.output("Result", int))
    page.blocks.append(block)
    outputs = await runner.run(block)
    assert outputs and outputs.Result == 12


async def test_run_code_function_coerce_tuple(runner: RuntimeRunner, page: Block):
    block = Block.new(
        BlockType.CODE,
        "Function2",
        code=code("""\
return Input1 > 10, Input1 * 4
"""),
    )
    block.fields.extend(
        Field.input("Input1", int), Field.output("Result1", bool), Field.output("Result2", int)
    )
    page.blocks.append(block)
    outputs = await runner.run(block)
    assert outputs and outputs.Result1 is False and outputs.Result2 == 12


async def test_run_code_function_coerce_dict(runner: RuntimeRunner, page: Block):
    block = Block.new(
        BlockType.CODE,
        "Function3",
        code=code("""\
return dict(a=Input1 > 10, b=Input1 * 4)
"""),
    )
    block.fields.extend(
        Field.input("Input1", int), Field.output("Result1", bool), Field.output("Result2", int)
    )
    page.blocks.append(block)
    outputs = await runner.run(block)
    assert outputs and outputs.Result1 is False and outputs.Result2 == 12
