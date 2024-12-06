import asyncio

from bench.language import Block, BlockType, Field, code
from bench.language.action import Agency
from bench.language.const import RunStatus
from bench.language.node import Node
from bench.language.run import Run, RunErrorType, RunOptions, RunType
from bench.runtime.capture import MAX_LOG_LINE_LENGTH, MAX_LOGS_PER_CAPTURE
from bench.runtime.core import ATTEMPT_ONCE
from bench.runtime.runner import make_run_from_node
from bench.test.unit.conftest import RuntimeHandle


async def test_run_code_function_empty(local_runtime: RuntimeHandle):
    """Empty Code with optional input/output Fields should work."""
    Code1 = Block.new(
        BlockType.ACTION,
        "Code1",
        fields=(Field.input("Input1", int), Field.output("Output1", int)),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Code1)
    await local_runtime.commit()

    runner = await local_runtime.run(Code1, return_error=True)
    assert runner.status == RunStatus.COMPLETED


async def test_run_code_with_syntax_error(local_runtime: RuntimeHandle):
    """Code block with a syntax error should re-raise that error."""
    InvalidCode = Block.new(
        BlockType.ACTION, "InvalidCode", code=code("!!invalid!!"), agency=Agency.CODE
    )
    local_runtime.page().blocks.append(InvalidCode)
    await local_runtime.commit()

    runner = await local_runtime.run(InvalidCode, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error is not None and runner.error.type == RunErrorType.CODE_INVALID
    assert runner.error.text and "!!invalid!!" in runner.error.text


async def test_run_code_capture_logs(local_runtime: RuntimeHandle):
    """All logging functions should be captured."""
    Logs101 = Block.new(
        BlockType.ACTION,
        "Logs101",
        code=code("""\
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
"""),
        agency=Agency.CODE,
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
    Logs102 = Block.new(
        BlockType.ACTION,
        "Logs102",
        code=code("""\
print('print1')
print('print2')
raise ValueError('error1')
print('print3')
"""),
        agency=Agency.CODE,
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
    Logs103 = Block.new(
        BlockType.ACTION,
        "Logs103",
        code=code(f"""\
for i in range(0, {MAX_LOGS_PER_CAPTURE + 5}):
    print('print', i)
"""),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Logs103)
    await local_runtime.commit()

    runner = await local_runtime.run(Logs103)
    assert len(runner.logs) == MAX_LOGS_PER_CAPTURE
    assert runner.logs[-1].text_plain and "overflow" in runner.logs[-1].text_plain


async def test_run_code_capture_log_line_overflow(local_runtime: RuntimeHandle):
    """Logs should only be captured up to a certain size."""
    Logs103 = Block.new(
        BlockType.ACTION,
        "Logs103",
        code=code(f"""print('x' * {MAX_LOG_LINE_LENGTH + 5})"""),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Logs103)
    await local_runtime.commit()

    runner = await local_runtime.run(Logs103)
    assert runner.logs and len(runner.logs) == 1
    assert runner.logs[0].text_plain and "truncate" in runner.logs[0].text_plain


async def test_run_code_function_invalid_inputs(local_runtime: RuntimeHandle):
    """Code block with invalid inputs should fail immediately (no attempts)."""
    Code1 = Block.new(
        BlockType.ACTION,
        "InvalidCode",
        code=code("pass"),
        fields=(Field.input("Input1", int, is_required=True),),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Code1)
    await local_runtime.commit()

    run = Run(parent=local_runtime.package, options=ATTEMPT_ONCE, type=RunType.CODE, block=Code1)
    runner = await local_runtime.run(run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert len(runner.attempts) == 0
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE
    assert runner.tracked_run and runner.tracked_run.duration is not None


async def test_run_code_function_invalid_outputs(local_runtime: RuntimeHandle):
    """Code block with invalid outputs should fail."""
    Code1 = Block.new(
        BlockType.ACTION,
        "InvalidCode",
        code=code("return 'invalid'"),
        fields=(Field.output("Output1", int),),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Code1)
    await local_runtime.commit()

    run = Run(parent=local_runtime.package, options=ATTEMPT_ONCE, type=RunType.CODE, block=Code1)
    runner = await local_runtime.run(run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE


async def test_run_code_function_coerce_inputs(local_runtime: RuntimeHandle):
    """All the input fields values should be coerced to the correct type."""
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("return Input1, Input2"),
        fields=(
            Field.input("Input1", int),
            Field.input("Input2", int),
            Field.output("Output1", float),
            Field.output("Output2", float),
        ),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function, inputs={"Input1": 3.0, "Input2": 4.4})
    assert runner.outputs and runner.outputs.Output1 == 3 and runner.outputs.Output2 == 4


async def test_run_code_function_inputs_in_context(local_runtime: RuntimeHandle):
    """All the input fields values should be in context (even if not used and unset)."""
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("""\
assert Input1 == 3
assert Input2 is None
assert Long_Input == "hi"
assert Very_WEIRD__THER_Input == 7
"""),
        fields=(
            Field.input("Input1", int),
            Field.input("Input2", bool),
            Field.input("Long Input", str),
            Field.input("Very WEIRD ÖTHER Input", int),
        ),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    _ = await local_runtime.run(
        Function, inputs={"Input1": 3, "Long Input": "hi", "Very WEIRD ÖTHER Input": 7}
    )


async def test_run_code_function_output_none(local_runtime: RuntimeHandle):
    """A noop code function should work and return None."""
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("""pass"""),
        fields=(Field.input("Input1", int),),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    _ = await local_runtime.run(Function)


async def test_run_code_function_output_scalar(hosted_runtime: RuntimeHandle):
    """
    Run a code function with a scalar, should coerce into object.
    NOTE: we use the hosted_runtime here as we edit the node subtype property CodeBlock.code
     (and the local runtime works directly in the SQL engine, which can't do hierarchical edits)
    """
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("""return Input1 * 4"""),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
        agency=Agency.CODE,
    )
    hosted_runtime.page().blocks.append(Function)
    await hosted_runtime.commit()

    # run with good return value
    runner = await hosted_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 == 12

    # run with cast return value
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("return Input1 * 1.7"),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
        agency=Agency.CODE,
    )
    hosted_runtime.page().blocks.append(Function)
    await hosted_runtime.commit()
    runner = await hosted_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 == 5

    # run with bad return value
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("return 'stringy'"),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
        agency=Agency.CODE,
    )
    hosted_runtime.page().blocks.append(Function)
    await hosted_runtime.commit()
    runner = await hosted_runtime.run(Function, inputs={"Input1": 3}, return_error=True)
    assert runner.status == RunStatus.FAILED


async def test_run_code_function_output_tuple(local_runtime: RuntimeHandle):
    """Run a code function with a tuple, should coerce into object."""
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("""return Input1 > 10, Input1 * 4, None"""),
        fields=(
            Field.input("Input1", int),
            Field.output("Result1", bool, is_required=True),
            Field.output("Result2", int),
            Field.output("Result3", int),
        ),
        agency=Agency.CODE,
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
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("""return dict(Result1=Input1 > 10, Result2=Input1 * 4)"""),
        fields=(
            Field.input("Input1", int),
            Field.output("Result1", bool),
            Field.output("Result2", int),
        ),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 is False and runner.outputs.Result2 == 12


async def test_run_code_function_output_object_raw(local_runtime: RuntimeHandle):
    """Run a code function and return the output object directly."""
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("""\
return coerce_custom_object(
    kind=ObjectKind.OUTPUT, 
    value_raw={"Input1": 1, "Input2": 2},
    typ=self.to_type(of='value', field_type=FieldType.OUTPUT), 
)
"""),
        fields=[Field.output("Input1", int), Field.output("Input2", int)],
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function, inputs={})
    assert runner.outputs and runner.outputs.Input1 == 1 and runner.outputs.Input2 == 2


async def test_run_code_function_output_choice(local_runtime: RuntimeHandle):
    """Run a code function with a dict and a Choice type, should coerce into object."""
    Color = Block.new(
        BlockType.CHOICE,
        "Color",
        fields=[Field.option("Red"), Field.option("Green"), Field.option("Blue")],
    )
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("""\
Color = get_node("^Color")
return {"Color": Color.Red}
"""),
        fields=[Field.output("Color", Color)],
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.extend(Color, Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function)
    assert runner.outputs and runner.outputs.Color == Color.fields.Red


async def test_run_code_function_output_generic_node(local_runtime: RuntimeHandle):
    """Run a code function that outputs a generic node field."""
    Function = Block.new(
        BlockType.ACTION,
        "Function",
        code=code("""\
return [self]
"""),
        fields=[Field.output("Output", Node, is_list=True)],
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function)
    assert runner.outputs


async def test_run_code_raise_retryable_error(local_runtime: RuntimeHandle):
    """Raise a retryable error. Should be detected and retried."""
    CodeBlock = Block.new(
        BlockType.ACTION,
        "Code1",
        code=code("""raise RetryableError('error1')"""),
        agency=Agency.CODE,
        run_options=RunOptions(max_attempts=3),
    )
    local_runtime.page().blocks.append(CodeBlock)
    await local_runtime.commit()

    runner = await local_runtime.run(CodeBlock, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.RETRYABLE
    assert len(runner.attempts) == 3


async def test_run_code_raise_unretryable_error(local_runtime: RuntimeHandle):
    """Raise an unretryable error. Should be detected and not retried."""
    CodeBlock = Block.new(
        BlockType.ACTION,
        "Code1",
        code=code("""raise NonRetryableError('error1')"""),
        agency=Agency.CODE,
        run_options=RunOptions(max_attempts=3),
    )
    local_runtime.page().blocks.append(CodeBlock)
    await local_runtime.commit()

    runner = await local_runtime.run(CodeBlock, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.NON_RETRYABLE
    assert len(runner.attempts) == 1


async def test_run_code_abort(local_runtime: RuntimeHandle):
    """Run a long async code script and abort it."""
    CodeBlock = Block.new(
        BlockType.ACTION,
        "Code1",
        code=code("""await asyncio.sleep(5)"""),
        agency=Agency.CODE,
    )
    local_runtime.page().blocks.append(CodeBlock)
    await local_runtime.commit()

    run = make_run_from_node(CodeBlock)
    run_task = asyncio.create_task(local_runtime.run(run, return_error=True))
    # kill after 0.5s
    await asyncio.sleep(0.5)
    local_runtime.runtime.stop(run)
    runner = await run_task
    # run should be aborted
    assert runner.status == RunStatus.ABORTED
    assert (
        runner.tracked_run
        and runner.tracked_run.duration
        and runner.tracked_run.duration.total_seconds() < 1
    )
