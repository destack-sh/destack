import pytest

from bench.language import Block, BlockType, Field, code
from bench.language.const import RunStatus
from bench.runtime.runner import RuntimeRunner


async def test_run_code_with_syntax_error(runner: RuntimeRunner, page: Block):
    block = Block.new(BlockType.CODE, "Invalid", code=code("!!invalid!!"))
    page.blocks.append(block)
    await runner.session.commit()

    with pytest.raises(SyntaxError):
        run = await runner.run(block)
        assert run.status == RunStatus.FAILED


async def test_run_code_function_coerce_single(runner: RuntimeRunner, page: Block):
    block = Block.new(
        BlockType.CODE,
        "Function1",
        code=code("""\
return Input1 * 4
"""),
    )
    block.fields.extend(Field.input("Input1", int), Field.output("Result1", int))
    page.blocks.append(block)
    await runner.session.commit()

    run = await runner.run(block, inputs={"Input1": 3})
    assert run.outputs and run.outputs.Result1 == 12


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
    await runner.session.commit()

    run = await runner.run(block, inputs={"Input1": 3})
    assert run.outputs and run.outputs.Result1 is False and run.outputs.Result2 == 12


async def test_run_code_function_coerce_dict(runner: RuntimeRunner, page: Block):
    block = Block.new(
        BlockType.CODE,
        "Function3",
        code=code("""\
return dict(Result1=Input1 > 10, Result2=Input1 * 4)
"""),
    )
    block.fields.extend(
        Field.input("Input1", int), Field.output("Result1", bool), Field.output("Result2", int)
    )
    page.blocks.append(block)
    await runner.session.commit()

    run = await runner.run(block, inputs={"Input1": 3})
    assert run.outputs and run.outputs.Result1 is False and run.outputs.Result2 == 12
