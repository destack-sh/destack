import pytest

from bench.language import Block, BlockType, Field, code
from bench.language.const import RunStatus
from bench.runtime.runner import RuntimeRunner


async def test_run_code_with_syntax_error(runner: RuntimeRunner, page: Block):
    InvalidCode = Block.new_code("InvalidCode", "!!invalid!!")
    page.blocks.append(InvalidCode)
    await runner.session.commit()

    with pytest.raises(SyntaxError):
        _ = await runner.run(InvalidCode)


async def test_run_code_capture_logs(runner: RuntimeRunner, page: Block):
    Logs101 = Block.new_code(
        "Logs101",
        """\
import builtins
print('print1', 'print2') # our own print (injected)
builtins.print('print3') # python print
log('log1')
trace('trace1')
builtins.print('print4') # python print
debug('debug1')
builtins.print('print5\\nwith newline') # python print
info('info1')
warn('warn1')
error('error1')
critical('critical1')        
""",
    )
    page.blocks.append(Logs101)
    await runner.session.commit()

    run = await runner.run(Logs101)
    assert run.status == RunStatus.COMPLETED
    assert run.logs and len(run.logs) == 11
    for s, log in zip(
        (
            "print1 print2",
            "print3",
            "log1",
            "trace1",
            "print4",
            "debug1",
            "print5\nwith newline",
            "info1",
            "warn1",
            "error1",
            "critical1",
        ),
        run.logs,
    ):
        assert log.text_plain == s


async def test_run_code_capture_logs_on_error(runner: RuntimeRunner, page: Block):
    Logs102 = Block.new_code(
        "Logs102",
        """\
print('print1')
print('print2')
raise ValueError('error1')
print('print3')
""",
    )
    page.blocks.append(Logs102)
    await runner.session.commit()

    run = await runner.run(Logs102, suppress_error=True)
    assert run.status == RunStatus.FAILED
    assert run.logs and len(run.logs) == 2
    for s, log in zip(("print1", "print2"), run.logs):
        assert log.text_plain == s


async def test_run_code_function_no_output(runner: RuntimeRunner, page: Block):
    Function = Block.new_code("Function", """pass""", fields=(Field.input("Input1", int),))
    page.blocks.append(Function)
    await runner.session.commit()

    _ = await runner.run(Function)


async def test_run_code_function_coerce_output_scalar(runner: RuntimeRunner, page: Block):
    Function = Block.new_code(
        "Function",
        """return Input1 * 4""",
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    page.blocks.append(Function)
    await runner.session.commit()

    run = await runner.run(Function, inputs={"Input1": 3})
    assert run.outputs and run.outputs.Result1 == 12

    # now with bad return value
    Function.code = code("return 'stringy'")
    await runner.session.commit()
    run = await runner.run(Function, inputs={"Input1": 3}, suppress_error=True)
    assert run.status == RunStatus.FAILED


async def test_run_code_function_coerce_output_tuple(runner: RuntimeRunner, page: Block):
    Function = Block.new_code(
        "Function",
        """return Input1 > 10, Input1 * 4, None""",
        fields=(
            Field.input("Input1", int),
            Field.output("Result1", bool, is_required=True),
            Field.output("Result2", int),
            Field.output("Result3", int),
        ),
    )
    page.blocks.append(Function)
    await runner.session.commit()

    run = await runner.run(Function, inputs={"Input1": 3})
    assert (
        run.outputs
        and run.outputs.Result1 is False
        and run.outputs.Result2 == 12
        and run.outputs.Result3 is None
    )


async def test_run_code_function_coerce_output_dict(runner: RuntimeRunner, page: Block):
    Function = Block.new_code(
        "Function",
        """return dict(Result1=Input1 > 10, Result2=Input1 * 4)""",
    )
    Function.fields.extend(
        Field.input("Input1", int), Field.output("Result1", bool), Field.output("Result2", int)
    )
    page.blocks.append(Function)
    await runner.session.commit()

    run = await runner.run(Function, inputs={"Input1": 3})
    assert run.outputs and run.outputs.Result1 is False and run.outputs.Result2 == 12


async def test_run_code_resolve_references(runner: RuntimeRunner, page: Block):
    subpage = page.blocks.append(Block.new(BlockType.PAGE, "Subpage"))
    ShapeKind = Block.new(
        BlockType.CHOICE,
        "ShapeKind",
        fields=[
            Field.option("Rectangle"),
            Field.option("Circle"),
            Field.option("Triangle"),
            Field.option("Square"),
        ],
    )
    Shape = Block.new(
        BlockType.CLASS,
        "Shape",
        fields=[Field.member("kind", ShapeKind)],
    )
    subpage.blocks.extend(ShapeKind, Shape)
    await runner.session.commit()

    Function = Block.new_code(
        "Function",
        """\
assert ShapeKind is not None
assert ShapeKind.fields.Circle is not None
assert ShapeKind.Rectangle is not None
assert Shape(kind=ShapeKind.Square)
""",
    )
    subpage.blocks.append(Function)

    _ = await runner.run(Function)
