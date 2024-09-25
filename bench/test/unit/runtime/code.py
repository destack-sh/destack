import asyncio

from bench.language import Block, BlockType, Field, code
from bench.language.const import RunStatus
from bench.language.run import Run, RunErrorType, RunKind, RunOptions
from bench.language.value import ValueObject
from bench.runtime.capture import MAX_LOG_LINE_LENGTH, MAX_LOGS_PER_CAPTURE
from bench.test.unit.conftest import RuntimeHandle


async def test_run_code_script_empty(local_runtime: RuntimeHandle):
    """Empty Code without any fields should fail."""
    Code1 = Block.new(BlockType.CODE, "Code1")
    local_runtime.page().blocks.append(Code1)
    await local_runtime.commit()

    _ = await local_runtime.run(Code1)


async def test_run_code_function_empty(local_runtime: RuntimeHandle):
    """Empty Code without any fields should fail."""
    Code1 = Block.new(
        BlockType.CODE, "Code1", fields=(Field.input("Input1", int), Field.output("Output1", int))
    )
    local_runtime.page().blocks.append(Code1)
    await local_runtime.commit()

    _ = await local_runtime.run(Code1)


async def test_run_code_with_syntax_error(local_runtime: RuntimeHandle):
    """Code block with a syntax error should re-raise that error."""
    InvalidCode = Block.new_code("InvalidCode", "!!invalid!!")
    local_runtime.page().blocks.append(InvalidCode)
    await local_runtime.commit()

    runner = await local_runtime.run(InvalidCode, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error is not None and runner.error.type == RunErrorType.CODE_INVALID
    assert runner.error.text and "!!invalid!!" in runner.error.text


async def test_run_code_capture_logs(local_runtime: RuntimeHandle):
    """All logging functions should be captured."""
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

    runner = await local_runtime.run(Logs101)
    assert runner.status == RunStatus.COMPLETED
    assert runner.logs and len(runner.logs) == 11
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
        runner.logs,
    ):
        assert log.text_plain == s


async def test_run_code_capture_logs_on_error(local_runtime: RuntimeHandle):
    """Logs should also be captured if the code raises an error."""
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

    runner = await local_runtime.run(Logs102, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.logs and len(runner.logs) == 2
    for s, log in zip(("print1", "print2"), runner.logs):
        assert log.text_plain == s


async def test_run_code_capture_log_size_overflow(local_runtime: RuntimeHandle):
    """Logs should only be captured up to a certain size."""
    Logs103 = Block.new_code(
        "Logs103",
        f"""\
for i in range(0, {MAX_LOGS_PER_CAPTURE + 5}):
    print('print', i)
""",
    )
    local_runtime.page().blocks.append(Logs103)
    await local_runtime.commit()

    runner = await local_runtime.run(Logs103)
    assert len(runner.logs) == MAX_LOGS_PER_CAPTURE
    assert runner.logs[-1].text_plain and "overflow" in runner.logs[-1].text_plain


async def test_run_code_capture_log_line_overflow(local_runtime: RuntimeHandle):
    """Logs should only be captured up to a certain size."""
    Logs103 = Block.new_code("Logs103", f"""print('x' * {MAX_LOG_LINE_LENGTH + 5})""")
    local_runtime.page().blocks.append(Logs103)
    await local_runtime.commit()

    runner = await local_runtime.run(Logs103)
    assert runner.logs and len(runner.logs) == 1
    assert runner.logs[0].text_plain and "truncate" in runner.logs[0].text_plain


async def test_run_code_function_invalid_inputs(local_runtime: RuntimeHandle):
    """Code block with invalid inputs should fail immediately (no attempts)."""
    Code1 = Block.new_code(
        "InvalidCode", "pass", fields=(Field.input("Input1", int, is_required=True),)
    )
    local_runtime.page().blocks.append(Code1)
    await local_runtime.commit()

    run = Run(parent=local_runtime.package, kind=RunKind.CODE, block=Code1)
    runner = await local_runtime.run(run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert len(runner.attempts) == 0
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE
    assert runner.run and runner.run.duration is not None


async def test_run_code_function_invalid_outputs(local_runtime: RuntimeHandle):
    """Code block with invalid outputs should fail."""
    Code1 = Block.new_code(
        "InvalidCode", "return 'invalid'", fields=(Field.output("Output1", int),)
    )
    local_runtime.page().blocks.append(Code1)
    await local_runtime.commit()

    run = Run(parent=local_runtime.package, kind=RunKind.CODE, block=Code1)
    runner = await local_runtime.run(run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE


async def test_run_code_function_coerce_inputs(local_runtime: RuntimeHandle):
    """All the input fields values should be coerced to the correct type."""
    Function = Block.new_code(
        "Function",
        "return Input1, Input2",
        fields=(
            Field.input("Input1", int),
            Field.input("Input2", int),
            Field.output("Output1", float),
            Field.output("Output2", float),
        ),
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function, inputs={"Input1": 3.0, "Input2": 4.4})
    assert runner.outputs and runner.outputs.Output1 == 3 and runner.outputs.Output2 == 4


async def test_run_code_function_inputs_in_context(local_runtime: RuntimeHandle):
    """All the input fields values should be in context (even if not used and unset)."""
    Function = Block.new_code(
        "Function",
        """\
assert Input1 == 3
assert Input2 is None
assert Long_Input == "hi"
assert Very_WEIRD__THER_Input == 7
""",
        fields=(
            Field.input("Input1", int),
            Field.input("Input2", bool),
            Field.input("Long Input", str),
            Field.input("Very WEIRD ÖTHER Input", int),
        ),
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    _ = await local_runtime.run(
        Function, inputs={"Input1": 3, "Long Input": "hi", "Very WEIRD ÖTHER Input": 7}
    )


async def test_run_code_function_output_none(local_runtime: RuntimeHandle):
    """A noop code function should work and return None."""
    Function = Block.new_code("Function", """pass""", fields=(Field.input("Input1", int),))
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    _ = await local_runtime.run(Function)


async def test_run_code_function_output_scalar(local_runtime: RuntimeHandle):
    """Run a code function with a scalar, should coerce into object."""
    Function = Block.new_code(
        "Function",
        """return Input1 * 4""",
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    # run with good return value
    runner = await local_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 == 12

    # run with cast return value
    Function.code = code("return Input1 * 1.7")
    await local_runtime.commit()
    runner = await local_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 == 5

    # run with bad return value
    Function.code = code("return 'stringy'")
    await local_runtime.commit()
    runner = await local_runtime.run(Function, inputs={"Input1": 3}, return_error=True)
    assert runner.status == RunStatus.FAILED


async def test_run_code_function_output_tuple(local_runtime: RuntimeHandle):
    """Run a code function with a tuple, should coerce into object."""
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

    runner = await local_runtime.run(Function, inputs={"Input1": 3})
    assert (
        runner.outputs
        and runner.outputs.Result1 is False
        and runner.outputs.Result2 == 12
        and runner.outputs.Result3 is None
    )


async def test_run_code_function_output_dict(local_runtime: RuntimeHandle):
    """Run a code function with a dict, should coerce into object."""
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

    runner = await local_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 is False and runner.outputs.Result2 == 12


async def test_run_code_function_output_choice(local_runtime: RuntimeHandle):
    """Run a code function with a dict and a Choice type, should coerce into object."""
    Color = Block.new(
        BlockType.CHOICE,
        "Color",
        fields=[Field.option("Red"), Field.option("Green"), Field.option("Blue")],
    )
    Function = Block.new_code(
        "Function",
        """\
Color = get_node("^Color")
return {"Color": Color.Red}
""",
        fields=[Field.output("Color", Color)],
    )
    local_runtime.page().blocks.extend(Color, Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function)
    assert runner.outputs and runner.outputs.Color == Color.fields.Red


async def test_run_code_function_output_nested(local_runtime: RuntimeHandle):
    """ "Run a code function with a nested object."""
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

    runner = await local_runtime.run(Function)
    assert runner.outputs and isinstance(runner.outputs.Result, ValueObject)
    assert runner.outputs.Result.kind == ShapeKind.fields.Square


async def test_run_code_raise_retryable_error(local_runtime: RuntimeHandle):
    """Raise a retryable error. Should be detected and retried."""
    CodeBlock = Block.new_code(
        "Code1", """raise RetryableError('error1')""", run_options=RunOptions(max_attempts=3)
    )
    local_runtime.page().blocks.append(CodeBlock)
    await local_runtime.commit()

    runner = await local_runtime.run(CodeBlock, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.UNKNOWN_RETRYABLE
    assert len(runner.attempts) == 3


async def test_run_code_raise_unretryable_error(local_runtime: RuntimeHandle):
    """Raise an unretryable error. Should be detected and not retried."""
    CodeBlock = Block.new_code(
        "Code1", """raise NonRetryableError('error1')""", run_options=RunOptions(max_attempts=3)
    )
    local_runtime.page().blocks.append(CodeBlock)
    await local_runtime.commit()

    runner = await local_runtime.run(CodeBlock, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.UNKNOWN_NONRETRYABLE
    assert len(runner.attempts) == 1


async def test_run_code_abort(local_runtime: RuntimeHandle):
    """Run a long async code script and abort it."""
    CodeBlock = Block.new_code(
        "Code1",
        """await asyncio.sleep(5)""",
    )
    local_runtime.page().blocks.append(CodeBlock)
    await local_runtime.commit()

    run = Run.from_runnable(CodeBlock)
    run_task = asyncio.create_task(local_runtime.run(run, return_error=True))
    # kill after 0.5s
    await asyncio.sleep(0.5)
    await local_runtime.runtime.abort_run(run)
    runner = await run_task
    # run should be aborted
    assert runner.status == RunStatus.ABORTED
    assert runner.run and runner.run.duration and runner.run.duration < 1
