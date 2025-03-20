from bench.language import (
    Action,
    ActionType,
    Block,
    Code,
    ErrorType,
    Field,
    Flow,
    Node,
    RunOptions,
    RunStatus,
    Text,
    TextLine,
    code,
)
from bench.runtime import MAX_LOG_LINE_LENGTH, MAX_LOGS_PER_RUN, create_run
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@simulated_runtime()
async def test_run_code_empty(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Empty Code with optional input/output Fields should work."""
    Flow1 = Flow.new("Flow1")
    Code1 = Action.new(
        ActionType.CODE,
        "Code1",
        fields=(Field.input("Input1", int), Field.output("Output1", int)),
    )
    Flow1.actions.append(Code1)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Code1, return_error=True)
    assert runner.status == RunStatus.COMPLETED


@simulated_runtime()
async def test_run_code_with_syntax_error(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Code Action with a syntax error should re-raise that error (at runtime)."""
    Flow1 = Flow.new("Flow1")
    InvalidCode = Action.new(ActionType.CODE, "InvalidCode", code=code("!!invalid!!"))
    Flow1.actions.append(InvalidCode)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(InvalidCode, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error is not None and runner.error.type == ErrorType.CODE_INVALID
    assert runner.error.text and "!!invalid!!" in runner.error.text


@simulated_runtime()
async def test_run_code_capture_logs(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """All logging functions should be captured."""
    Flow1 = Flow.new("Flow1")
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
panic('panic1')        
"""),
    )
    Flow1.actions.append(Logs101)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Logs101)
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
            "panic1",
        ),
        runner.logs,
    ):
        assert log.title == s


@simulated_runtime()
async def test_run_code_capture_logs_on_error(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Logs should also be captured if the code raises an error."""
    Flow1 = Flow.new("Flow1")
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
    Flow1.actions.append(Logs102)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Logs102, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.logs and len(runner.logs) == 2
    for s, log in zip(("print1", "print2"), runner.logs):
        assert log.title == s


@simulated_runtime()
async def test_run_code_capture_log_size_overflow(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Logs should only be captured up to a certain size."""
    Flow1 = Flow.new("Flow1")
    Logs103 = Action.new(
        ActionType.CODE,
        "Logs103",
        code=code(f"""\
for i in range(0, {MAX_LOGS_PER_RUN + 5}):
    print('print', i)
"""),
    )
    Flow1.actions.append(Logs103)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Logs103)
    assert len(runner.logs) == MAX_LOGS_PER_RUN


@simulated_runtime()
async def test_run_code_capture_log_line_overflow(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Logs should only be captured up to a certain size."""
    Flow1 = Flow.new("Flow1")
    Logs103 = Action.new(
        ActionType.CODE,
        "Logs103",
        code=code(f"""print('x' * {MAX_LOG_LINE_LENGTH + 5})"""),
    )
    Flow1.actions.append(Logs103)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Logs103)
    assert runner.logs and len(runner.logs) == 1
    assert runner.logs[0].title and "truncate" in runner.logs[0].title


@simulated_runtime()
async def test_run_code_invalid_inputs(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Code block with invalid inputs should fail immediately (no attempts)."""
    Flow1 = Flow.new("Flow1")
    Code1 = Action.new(
        ActionType.CODE,
        "InvalidCode",
        code=code("pass"),
        fields=(Field.input("Input1", int, is_required=True),),
    )
    Flow1.actions.append(Code1)
    runtime.page().append(Flow1)
    await runtime.commit()

    run, _ = create_run(Code1, parent=runtime.main_package)
    runner = await runtime.run_in_runtime(run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert len(runner.attempts) == 0
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE
    assert runner.tracked_run and runner.tracked_run.duration is not None


@simulated_runtime()
async def test_run_code_invalid_outputs(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Code block with invalid outputs should fail."""
    Flow1 = Flow.new("Flow1")
    Code1 = Action.new(
        ActionType.CODE,
        "InvalidCode",
        code=code("return 'invalid'"),
        fields=(Field.output("Output1", int),),
    )
    Flow1.actions.append(Code1)
    runtime.page().append(Flow1)
    await runtime.commit()

    run, _ = create_run(Code1, parent=runtime.main_package)
    runner = await runtime.run_in_runtime(run, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE


@simulated_runtime()
async def test_run_code_coerce(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Inputs and outputs should be coerced to the correct type (if possible)."""
    Flow1 = Flow.new("Flow1")
    Code1 = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""\
return {
    "Output1": Input1,
    "Output2": Input2,
    "Output3": Input3,
    "Output4": Input4,
    "Output5": Input5,
}
"""),
        fields=(
            Field.input("Input1", int),
            Field.input("Input2", float),
            Field.input("Input3", str),
            Field.input("Input4", Code),
            Field.input("Input5", Text),
            Field.output("Output1", float),
            Field.output("Output2", int),
            Field.output("Output3", Code),
            Field.output("Output4", Text),
            Field.output("Output5", str),
        ),
    )
    Flow1.actions.append(Code1)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(
        Code1,
        inputs={"Input1": 3.3, "Input2": 4, "Input3": "hello", "Input4": "world", "Input5": "!"},
    )
    assert runner.outputs
    assert runner.outputs.Output1 == 3.0
    assert runner.outputs.Output2 == 4
    assert runner.outputs.Output3 == code("hello")
    assert runner.outputs.Output4 == Text(lines=[TextLine.code("world")])
    assert runner.outputs.Output5 == "!"


@simulated_runtime()
async def test_run_code_inputs_in_context(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """All the input fields values should be in context (even if not used and unset)."""
    Flow1 = Flow.new("Flow1")
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
    Flow1.actions.append(Function)
    runtime.page().append(Flow1)
    await runtime.commit()

    _ = await runtime.run_in_runtime(
        Function, inputs={"Input1": 3, "Long Input": "hi", "Very WEIRD ÖTHER Input": 7}
    )


@simulated_runtime()
async def test_run_code_output_none(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """A noop code function should work and return None."""
    Flow1 = Flow.new("Flow1")
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""pass"""),
        fields=(Field.input("Input1", int),),
    )
    Flow1.actions.append(Function)
    runtime.page().append(Flow1)
    await runtime.commit()

    _ = await runtime.run_in_runtime(Function)


@simulated_runtime()
async def test_run_code_output_scalar(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """
    Run a code function with a scalar, should coerce into object.
    NOTE: we use hosted_runtime here as we edit the node subtype property ActionBlock.code
     (and the local runtime works directly in the SQL engine, which can't do hierarchical edits)
    """
    Flow1 = Flow.new("Flow1")
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""return {"Result1": Input1 * 4}"""),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    Flow1.actions.append(Function)
    runtime.page().append(Flow1)
    await runtime.commit()

    # run with good return value
    runner = await runtime.run_in_runtime(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 == 12

    # run with coercible return value
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("return {'Result1': Input1 * 1.7}"),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    Flow1.actions.append(Function)
    await runtime.commit()
    runner = await runtime.run_in_runtime(Function, inputs={"Input1": 3})
    assert runner.outputs and runner.outputs.Result1 == 5

    # run with bad return value
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("return {'Result1': 'stringy'}"),
        fields=(Field.input("Input1", int), Field.output("Result1", int, is_required=True)),
    )
    Flow1.actions.append(Function)
    await runtime.commit()
    runner = await runtime.run_in_runtime(Function, inputs={"Input1": 3}, return_error=True)
    assert runner.status == RunStatus.FAILED


@simulated_runtime()
async def test_run_code_output_generic_node(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a code function that outputs a generic node field."""
    Flow1 = Flow.new("Flow1")
    Function = Action.new(
        ActionType.CODE,
        "Function",
        code=code("""\
return {"Output": [self]}
"""),
        fields=[Field.output("Output", Node, is_list=True)],
    )
    Flow1.actions.append(Function)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Function)
    assert runner.outputs


@simulated_runtime()
async def test_run_code_return_detached_node(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Run a code function that returns a detached Node. Should error."""
    Flow1 = Flow.new("Flow1")
    Function = Action.new(
        ActionType.CODE,
        "Function",
        fields=[Field.output("Output", Block), Field.output("Text", Text)],
    )
    Flow1.actions.append(Function)
    runtime.page().append(Flow1)
    await runtime.commit()

    # detached top-level node
    Function.code = code("return {'Output': Block.new(BlockType.PARAGRAPH)}")
    await runtime.commit()
    runner = await runtime.run_in_runtime(Function, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE

    # detached nested node
    Function.code = code("""\
Detached = Block.new(BlockType.PARAGRAPH)
text = Text(lines=[TextLine.plain('line1'), TextLine.new(TextLineType.PARAGRAPH, TextSpan.new(node=Detached))])
return {'Text': text}
""")
    await runtime.commit()
    runner = await runtime.run_in_runtime(Function, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.INVALID_VALUE


@simulated_runtime()
async def test_run_code_raise_retryable_error(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Raise a retryable error. Should be detected and retried."""
    Flow1 = Flow.new("Flow1")
    Action1 = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("""raise RetryableError('error1')"""),
        options=RunOptions(max_attempts=3),
    )
    Flow1.actions.append(Action1)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Action1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.RETRYABLE
    assert len(runner.attempts) == 3


@simulated_runtime()
async def test_run_code_raise_unretryable_error(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Raise an unretryable error. Should be detected and not retried."""
    Flow1 = Flow.new("Flow1")
    Action1 = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("""raise NonRetryableError('error1')"""),
        options=RunOptions(max_attempts=3),
    )
    Flow1.actions.append(Action1)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Action1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == ErrorType.NON_RETRYABLE
    assert len(runner.attempts) == 1
