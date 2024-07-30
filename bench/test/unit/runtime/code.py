import pytest

from bench.language import Block, BlockType, Field, code
from bench.language.const import RunStatus
from bench.language.run import RunErrorType, RunOptions
from bench.language.value import ValueObject
from bench.runtime.capture import MAX_LOG_LINE_LENGTH, MAX_LOGS_PER_CAPTURE
from bench.test.unit.conftest import RuntimeHandle


async def test_run_code_empty(local_runtime: RuntimeHandle):
    """Empty Code without any fields should fail."""
    Code1 = Block.new(BlockType.CODE, "Code1")
    local_runtime.page().blocks.append(Code1)
    await local_runtime.commit()

    _ = await local_runtime.run(Code1)


async def test_run_code_with_syntax_error(local_runtime: RuntimeHandle):
    InvalidCode = Block.new_code("InvalidCode", "!!invalid!!")
    local_runtime.page().blocks.append(InvalidCode)
    await local_runtime.commit()

    with pytest.raises(SyntaxError):
        _ = await local_runtime.run(InvalidCode)


async def test_run_code_capture_logs(local_runtime: RuntimeHandle):
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
    local_runtime.page().blocks.append(Logs101)
    await local_runtime.commit()

    run = await local_runtime.run(Logs101)
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


async def test_run_code_capture_logs_on_error(local_runtime: RuntimeHandle):
    Logs102 = Block.new_code(
        "Logs102",
        """\
print('print1')
print('print2')
raise ValueError('error1')
print('print3')
""",
    )
    local_runtime.page().blocks.append(Logs102)
    await local_runtime.commit()

    run = await local_runtime.run(Logs102, return_error=True)
    assert run.status == RunStatus.FAILED
    assert run.logs and len(run.logs) == 2
    for s, log in zip(("print1", "print2"), run.logs):
        assert log.text_plain == s


async def test_run_code_capture_log_size_overflow(local_runtime: RuntimeHandle):
    Logs103 = Block.new_code(
        "Logs103",
        f"""\
for i in range(0, {MAX_LOGS_PER_CAPTURE + 5}):
    print('print', i)
""",
    )
    local_runtime.page().blocks.append(Logs103)
    await local_runtime.commit()

    run = await local_runtime.run(Logs103)
    assert len(run.logs) == MAX_LOGS_PER_CAPTURE
    assert run.logs[-1].text_plain and "overflow" in run.logs[-1].text_plain


async def test_run_code_capture_log_line_overflow(local_runtime: RuntimeHandle):
    Logs103 = Block.new_code("Logs103", f"""print('x' * {MAX_LOG_LINE_LENGTH + 5})""")
    local_runtime.page().blocks.append(Logs103)
    await local_runtime.commit()

    run = await local_runtime.run(Logs103)
    assert run.logs and len(run.logs) == 1
    assert run.logs[0].text_plain and "truncate" in run.logs[0].text_plain


async def test_run_code_function_output_none(local_runtime: RuntimeHandle):
    Function = Block.new_code("Function", """pass""", fields=(Field.input("Input1", int),))
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    _ = await local_runtime.run(Function)


async def test_run_code_function_output_scalar(local_runtime: RuntimeHandle):
    Function = Block.new_code(
        "Function",
        """return Input1 * 4""",
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    run = await local_runtime.run(Function, inputs={"Input1": 3})
    assert run.outputs and run.outputs.Result1 == 12

    # now with bad return value
    Function.code = code("return 'stringy'")
    await local_runtime.commit()
    run = await local_runtime.run(Function, inputs={"Input1": 3}, return_error=True)
    assert run.status == RunStatus.FAILED


async def test_run_code_function_output_tuple(local_runtime: RuntimeHandle):
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
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    run = await local_runtime.run(Function, inputs={"Input1": 3})
    assert (
        run.outputs
        and run.outputs.Result1 is False
        and run.outputs.Result2 == 12
        and run.outputs.Result3 is None
    )


async def test_run_code_function_output_dict(local_runtime: RuntimeHandle):
    Function = Block.new_code(
        "Function",
        """return dict(Result1=Input1 > 10, Result2=Input1 * 4)""",
        fields=(
            Field.input("Input1", int),
            Field.output("Result1", bool),
            Field.output("Result2", int),
        ),
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    run = await local_runtime.run(Function, inputs={"Input1": 3})
    assert run.outputs and run.outputs.Result1 is False and run.outputs.Result2 == 12


async def test_run_code_function_output_nested(local_runtime: RuntimeHandle):
    subpage = local_runtime.page().blocks.append(Block.new(BlockType.PAGE, "Subpage"))
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
    await local_runtime.commit()

    Function = Block.new_code(
        "Function",
        """\
assert ShapeKind is not None
assert ShapeKind.fields.Circle is not None
assert ShapeKind.Rectangle is not None
return Shape(kind=ShapeKind.Square)
""",
        fields=[Field.output("Result", Shape)],
    )
    subpage.blocks.append(Function)
    await local_runtime.commit()

    run = await local_runtime.run(Function)
    assert run.outputs and isinstance(run.outputs.Result, ValueObject)
    assert run.outputs.Result.kind == ShapeKind.fields.Square


async def test_run_code_raise_retryable_error(local_runtime: RuntimeHandle):
    CodeBlock = Block.new_code(
        "Code1", """raise RetryableError('error1')""", run_options=RunOptions(max_attempts=3)
    )
    local_runtime.page().blocks.append(CodeBlock)
    await local_runtime.commit()

    run = await local_runtime.run(CodeBlock, return_error=True)
    assert run.status == RunStatus.FAILED
    assert run.error and run.error.type == RunErrorType.UNKNOWN_RETRYABLE
    assert len(run.attempts) == 3


async def test_run_code_raise_unretryable_error(local_runtime: RuntimeHandle):
    CodeBlock = Block.new_code(
        "Code1", """raise NonRetryableError('error1')""", run_options=RunOptions(max_attempts=3)
    )
    local_runtime.page().blocks.append(CodeBlock)
    await local_runtime.commit()

    run = await local_runtime.run(CodeBlock, return_error=True)
    assert run.status == RunStatus.FAILED
    assert run.error and run.error.type == RunErrorType.UNKNOWN_NONRETRYABLE
    assert len(run.attempts) == 1
