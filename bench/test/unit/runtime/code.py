import asyncio

from bench.language import Action, ActionType, Field, code
from bench.language.block import Block
from bench.language.const import BlockType, RunStatus
from bench.language.node import Node
from bench.language.run import RunErrorType, RunOptions
from bench.language.text import Text
from bench.runtime.capture import MAX_LOG_LINE_LENGTH, MAX_LOGS_PER_CAPTURE
from bench.runtime.runner import make_run_from_node
from bench.test.unit.conftest import RuntimeHandle


async def test_run_code_empty(local_runtime: RuntimeHandle):
    """Empty Code with optional input/output Fields should work."""
    Code1 = Action.new(
        ActionType.CODE,
        "Code1",
        fields=(Field.input("Input1", int), Field.output("Output1", int)),
    )
    local_runtime.page().actions.append(Code1)
    await local_runtime.commit()

    runner = await local_runtime.run(Code1, return_error=True)
    assert runner.status == RunStatus.COMPLETED


async def test_run_code_with_syntax_error(local_runtime: RuntimeHandle):
    """Code block with a syntax error should re-raise that error."""
    InvalidCode = Action.new(ActionType.CODE, "InvalidCode", code=code("!!invalid!!"))
    local_runtime.page().actions.append(InvalidCode)
    await local_runtime.commit()

    runner = await local_runtime.run(InvalidCode, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error is not None and runner.error.type == RunErrorType.CODE_INVALID
    assert runner.error.text and "!!invalid!!" in runner.error.text


async def test_run_code_capture_logs(local_runtime: RuntimeHandle):
    """All logging functions should be captured."""
    Logs101 = Action.new(
        ActionType.CODE,
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
    )
    local_runtime.page().actions.append(Logs101)
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
    Logs102 = Action.new(
        ActionType.CODE,
        "Logs102",
        code=code("""\
print('print1')
print('print2')
raise ValueError('error1')
print('print3')
"""),
    )
    local_runtime.page().actions.append(Logs102)
    await local_runtime.commit()

    runner = await local_runtime.run(Logs102, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.logs and len(runner.logs) == 2
    for s, log in zip(("print1", "print2"), runner.logs):
        assert log.text_plain == s


async def test_run_code_capture_log_size_overflow(local_runtime: RuntimeHandle):
    """Logs should only be captured up to a certain size."""
    Logs103 = Action.new(
        ActionType.CODE,
        "Logs103",
        code=code(f"""\
for i in range(0, {MAX_LOGS_PER_CAPTURE + 5}):
    print('print', i)
"""),
    )
    local_runtime.page().actions.append(Logs103)
    await local_runtime.commit()

    runner = await local_runtime.run(Logs103)
    assert len(runner.logs) == MAX_LOGS_PER_CAPTURE
    assert runner.logs[-1].text_plain and "overflow" in runner.logs[-1].text_plain


async def test_run_code_capture_log_line_overflow(local_runtime: RuntimeHandle):
    """Logs should only be captured up to a certain size."""
    Logs103 = Action.new(
        ActionType.CODE,
        "Logs103",
        code=code(f"""print('x' * {MAX_LOG_LINE_LENGTH + 5})"""),
    )
    local_runtime.page().actions.append(Logs103)
    await local_runtime.commit()

    runner = await local_runtime.run(Logs103)
    assert runner.logs and len(runner.logs) == 1
    assert runner.logs[0].text_plain and "truncate" in runner.logs[0].text_plain


async def test_run_code_invalid_inputs(local_runtime: RuntimeHandle):
    """Code block with invalid inputs should fail immediately (no attempts)."""
    Code1 = Action.new(
        ActionType.CODE,
        "InvalidCode",
        code=code("pass"),
        fields=(Field.input("Input1", int, is_required=True),),
    )
    local_runtime.page().actions.append(Code1)
    await local_runtime.commit()

    run = make_run_from_node(Code1)
    runner = await local_runtime.run(run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert len(runner.attempts) == 0
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE
    assert runner.tracked_run and runner.tracked_run.duration is not None


async def test_run_code_invalid_outputs(local_runtime: RuntimeHandle):
    """Code block with invalid outputs should fail."""
    Code1 = Action.new(
        ActionType.CODE,
        "InvalidCode",
        code=code("return 'invalid'"),
        fields=(Field.output("Output1", int),),
    )
    local_runtime.page().actions.append(Code1)
    await local_runtime.commit()

    run = make_run_from_node(Code1)
    runner = await local_runtime.run(run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE


async def test_run_code_coerce_inputs(local_runtime: RuntimeHandle):
    """All the input fields values should be coerced to the correct type."""
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("return {'Output1': Input1, 'Output2': Input2}"),
        fields=(
            Field.input("Input1", int),
            Field.input("Input2", int),
            Field.output("Output1", float),
            Field.output("Output2", float),
        ),
    )
    local_runtime.page().actions.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function, inputs={"Input1": 3.0, "Input2": 4.4})
    assert runner.outputs and runner.outputs.Output1 == 3 and runner.outputs.Output2 == 4


async def test_run_code_inputs_in_context(local_runtime: RuntimeHandle):
    """All the input fields values should be in context (even if not used and unset)."""
    Function = Action.new(
        ActionType.CODE,
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
    )
    local_runtime.page().actions.append(Function)
    await local_runtime.commit()

    _ = await local_runtime.run(
        Function, inputs={"Input1": 3, "Long Input": "hi", "Very WEIRD ÖTHER Input": 7}
    )


async def test_run_code_output_none(local_runtime: RuntimeHandle):
    """A noop code function should work and return None."""
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""pass"""),
        fields=(Field.input("Input1", int),),
    )
    local_runtime.page().actions.append(Function)
    await local_runtime.commit()

    _ = await local_runtime.run(Function)


async def test_run_code_output_scalar(hosted_runtime: RuntimeHandle):
    """
    Run a code function with a scalar, should coerce into object.
    NOTE: we use hosted_runtime here as we edit the node subtype property ActionBlock.code
     (and the local runtime works directly in the SQL engine, which can't do hierarchical edits)
    """
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""return {"Result1": Input1 * 4}"""),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    hosted_runtime.page().actions.append(Function)
    await hosted_runtime.commit()

    # run with good return value
    runner = await hosted_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 == 12

    # run with cast return value
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("return {'Result1': Input1 * 1.7}"),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    hosted_runtime.page().actions.append(Function)
    await hosted_runtime.commit()
    runner = await hosted_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 == 5

    # run with bad return value
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("return {'Result1': 'stringy'}"),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    hosted_runtime.page().actions.append(Function)
    await hosted_runtime.commit()
    runner = await hosted_runtime.run(Function, inputs={"Input1": 3}, return_error=True)
    assert runner.status == RunStatus.FAILED


async def test_run_code_output_tuple(local_runtime: RuntimeHandle):
    """Run a code function with a tuple, should coerce into object."""
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""return {"Result1": Input1 > 10, "Result2": Input1 * 4, "Result3": None}"""),
        fields=(
            Field.input("Input1", int),
            Field.output("Result1", bool, is_required=True),
            Field.output("Result2", int),
            Field.output("Result3", int),
        ),
    )
    local_runtime.page().actions.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function, inputs={"Input1": 3})
    assert (
        runner.outputs
        and runner.outputs.Result1 is False
        and runner.outputs.Result2 == 12
        and runner.outputs.Result3 is None
    )


async def test_run_code_output_dict(local_runtime: RuntimeHandle):
    """Run a code function with a dict, should coerce into object."""
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""return {"Result1": Input1 > 10, "Result2": Input1 * 4}"""),
        fields=(
            Field.input("Input1", int),
            Field.output("Result1", bool),
            Field.output("Result2", int),
        ),
    )
    local_runtime.page().actions.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 is False and runner.outputs.Result2 == 12


async def test_run_code_output_choice(local_runtime: RuntimeHandle):
    """Run a code function with a dict and a Choice type, should coerce into object."""
    Color = Block.new(
        BlockType.CHOICE,
        "Color",
        fields=[Field.option("Red"), Field.option("Green"), Field.option("Blue")],
    )
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""\
Color = get_node("^Color")
return {"Color": Color.Red}
"""),
        fields=[Field.output("Color", Color)],
    )
    local_runtime.page().blocks.extend(Color)
    local_runtime.page().actions.extend(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function)
    assert runner.outputs and runner.outputs.Color == Color.fields.Red


async def test_run_code_output_generic_node(local_runtime: RuntimeHandle):
    """Run a code function that outputs a generic node field."""
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""\
return {"Output": [self]}
"""),
        fields=[Field.output("Output", Node, is_list=True)],
    )
    local_runtime.page().actions.append(Function)
    await local_runtime.commit()

    runner = await local_runtime.run(Function)
    assert runner.outputs


async def test_run_code_return_detached_node(hosted_runtime: RuntimeHandle):
    """Run a code function that returns a detached Node. Should error."""
    Function = Action.new(
        ActionType.CODE,
        "Function",
        fields=[Field.output("Output", Block), Field.output("Text", Text)],
    )
    hosted_runtime.page().actions.append(Function)
    await hosted_runtime.commit()

    # detached top-level node
    Function.code = code("return {'Output': Block.new(BlockType.TEXT, 'Detached')}")
    await hosted_runtime.commit()
    runner = await hosted_runtime.run(Function, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE

    # detached nested node
    Function.code = code("""\
Detached = Block.new(BlockType.TEXT, 'Detached')
text = Text(lines=[TextLine.plain('line1'), TextLine.new(TextLineType.PLAIN, TextSpan.new(node=Detached))])
return {'Text': text}
""")
    await hosted_runtime.commit()
    runner = await hosted_runtime.run(Function, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.INVALID_VALUE


async def test_run_code_raise_retryable_error(local_runtime: RuntimeHandle):
    """Raise a retryable error. Should be detected and retried."""
    CodeBlock = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("""raise RetryableError('error1')"""),
        run_options=RunOptions(max_attempts=3),
    )
    local_runtime.page().actions.append(CodeBlock)
    await local_runtime.commit()

    runner = await local_runtime.run(CodeBlock, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.RETRYABLE
    assert len(runner.attempts) == 3


async def test_run_code_raise_unretryable_error(local_runtime: RuntimeHandle):
    """Raise an unretryable error. Should be detected and not retried."""
    CodeBlock = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("""raise NonRetryableError('error1')"""),
        run_options=RunOptions(max_attempts=3),
    )
    local_runtime.page().actions.append(CodeBlock)
    await local_runtime.commit()

    runner = await local_runtime.run(CodeBlock, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.NON_RETRYABLE
    assert len(runner.attempts) == 1


async def test_run_code_abort(local_runtime: RuntimeHandle):
    """Run a long async code script and abort it."""
    CodeBlock = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("""await asyncio.sleep(5)"""),
    )
    local_runtime.page().actions.append(CodeBlock)
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
