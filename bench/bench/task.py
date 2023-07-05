from __future__ import annotations

import abc
import enum
import json
import re
from json import JSONDecodeError
from typing import Collection, Optional, Self

from bench.bench import Code
from bench.bench.const import StatementType, TypeTag
from bench.bench.core import Scope, Statement, node
from bench.bench.expect import Expectation, HasExpectations
from bench.bench.model import Model
from bench.bench.type import HasType, Type, check_type, instantiate_py_value_flat, map_value
from bench.utils.utils import DotDict, DotDictList


class TaskErrorType(enum.StrEnum):
    INCAPABLE = "incapable"
    TIMEOUT = "timeout"
    INVALID_FORMAT = "invalid_format"
    INVALID_TYPE = "invalid_type"
    EXCEEDED_LIMIT = "exceeded_limit"
    UNKNOWN = "unknown"


class TaskError(ValueError):
    def __init__(self, type: TaskErrorType, message: str = None, path: str = None):
        super().__init__(message)
        self.type = type
        self.path = path

    @staticmethod
    def from_exception(e: Exception, path: str = None) -> TaskError:
        if isinstance(e, TaskError):
            return e
        elif isinstance(e, ValueError):
            return TaskError(TaskErrorType.INVALID_FORMAT, str(e), path)
        elif isinstance(e, TypeError):
            return TaskError(TaskErrorType.INVALID_TYPE, str(e), path)
        else:
            return TaskError(TaskErrorType.UNKNOWN, str(e), path)


class IncapableError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.INCAPABLE, message, path)


class LimitExceededError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.EXCEEDED_LIMIT, message, path)


@node(tracked=["description"])
class Task(HasType, HasExpectations, Statement):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.FUNCTION
    type: StatementType = StatementType.TASK
    _is_async: bool = True

    def _clear(self) -> None:
        Statement._clear(self)
        HasType._clear(self)
        HasExpectations._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        HasExpectations._interp(self, scope)

    async def __call__(
        self,
        *args,
        model: Model | list[Model] | str | list[str] = None,
        retries: int = None,
        cache: bool = None,
        timeout: float = None,
        batch: list[dict] = None,
        **kwargs,
    ):
        # map/batch inputs
        inputs = self._inputs_from_args(args, kwargs)
        is_batched = batch is not None
        if is_batched and not isinstance(batch, DotDictList):
            inputs = DotDictList(batch)
            del batch

        try:
            self.session.tracer.run_enter(self, inputs)
            if inputs and is_batched:
                raise ValueError("cannot specify both inputs and batch")

            # get models
            model = model or self.session.default_models
            if isinstance(model, str):
                model = self.session.module.lookup(model, statement_t=Model)
            elif isinstance(model, list):
                model = [
                    self.session.module.lookup(m, statement_t=Model) if isinstance(m, str) else m
                    for m in model
                ]
            if isinstance(model, Model):
                model = [model]

            # shortcut for built-in tasks with fixed implementations
            if self.path == "symbolx.lib.builtins.embed":
                ada = self.session.module.lookup("openai.lib.text.ada", statement_t=Model)
                if ada is None:
                    raise RuntimeError("openai.lib.text.ada not found")
                if is_batched:
                    ret = await ada(text=inputs["text"], cache=cache, timeout=timeout)
                else:
                    ret = await ada(**inputs, cache=cache, timeout=timeout)
            elif self.path == "symbolx.lib.builtins.transcribe":
                raise NotImplementedError
            else:
                if not model:
                    raise RuntimeError(f"{self} has no default models and none were specified")
                # TODO @Robustness: rotate models (on failure?)
                ret = await self._run_task_block(model[0], inputs, is_batched)
            self.session.tracer.run_exit(self, ret)
            return ret
        except Exception as e:
            self.session.tracer.run_exception(self, e)
            raise

    async def _run_task_block(self, model: Model, inputs: dict, is_batched: bool):
        """Runs a single contiguous 'block' of a task on a single model."""

        # compile
        compiler = model.compile(self, inputs, is_batched)
        for child in self.children:
            if isinstance(child, (Code, Task, Model)):
                compiler.add_function(child)
            elif isinstance(child, Expectation):
                compiler.add_expectation(child)
        for type in self.walk_type(include_references=False):
            pass  # nocheckin add all type instruction

        # run
        runner = TaskRunner(self, max_steps=10, max_function_calls=3, max_errors=3)
        ret = await compiler.run(model, runner)
        if isinstance(ret, TaskError):
            raise ret
        return ret

    def to_async(self) -> "Self":
        return self

    def to_sync(self) -> "Self":
        if self._is_async:
            return TaskProxy.to_sync(self)
        return self


class TaskProxy:  # :SyncProxy
    """A simple proxy for Task to enable to_sync/to_async while keeping the original Task object."""

    def __init__(self, task: Task, is_async: bool):
        self._task = task
        self._is_async = is_async
        self._task_callable_sync = None

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self._task(*args, **kwargs)
        else:
            if self._task_callable_sync is None:
                self._task_callable_sync = self._task.session.async_to_sync(self._task.__call__)
            return self._task_callable_sync(*args, **kwargs)

    def __getattr__(self, name):
        return getattr(self._task, name)

    def to_async(self) -> Task:
        return self.task

    @classmethod
    def to_sync(cls, task: Task) -> "TaskProxy":
        return cls(task, is_async=False)


class TaskRunner(abc.ABC):
    def __init__(self, task: Task, max_steps: int, max_function_calls: int, max_errors: int):
        self.task = task
        self.max_steps = max_steps
        self.max_function_calls = max_function_calls
        self.max_errors = max_errors
        self.num_steps = 0
        self.num_function_calls = 0
        self.num_errors = 0

    async def step(self):
        self.num_steps += 1
        if self.num_steps > self.max_steps:
            raise LimitExceededError("max steps exceeded")

    async def error(self, error: TaskError):
        self.num_errors += 1
        if self.num_errors > self.max_errors:
            raise LimitExceededError("max errors exceeded")

    @property
    def can_call_another_function(self) -> bool:
        return self.num_function_calls < self.max_function_calls

    async def call_function(self, function: Task | Code | Model, inputs: dict) -> dict | TaskError:
        try:
            self.num_function_calls += 1
            if self.num_function_calls > self.max_function_calls:
                raise LimitExceededError("max function calls exceeded")
            return await function.to_async()(**inputs)
        except (IncapableError, ValueError, TypeError) as e:
            return TaskError.from_exception(e)


class TaskCompiler(abc.ABC):
    def __init__(self, task: Task, inputs: dict | list[dict], is_batched: bool):
        self.task = task
        self.inputs = inputs
        self.is_batched = is_batched
        self.steps: list[Task] = []
        self.functions_by_py_ident: dict[str, Task | Code | Model] = {}
        self.expectations: list[Expectation] = []

    @property
    def functions(self) -> Collection[Task | Code | Model]:
        return self.functions_by_py_ident.values()

    def add_step(self, step: Task) -> None:
        self.steps.append(step)

    def add_function(self, function: Task | Code | Model) -> None:
        self.functions_by_py_ident[function.py_ident] = function

    def add_expectation(self, expectation: Expectation) -> None:
        self.expectations.append(expectation)

    async def run(self, model: Model, runner: TaskRunner) -> dict | TaskError:
        """Runs the compiled task and returns the result"""
        raise NotImplementedError


def _parse_string_output(output: str, type: Type):
    """Parse json output from a string."""

    # escape/try to parse the output if needed (handles trivial model confusions)
    value = output.strip()
    if not value.startswith("{"):
        # sometimes the model prefixes the output with some explanation, find the { ... }
        value = re.compile(r"\{.*}", re.DOTALL).search(value)
        if value:
            value = value.group(0)
        else:
            raise TaskError(
                TaskErrorType.INVALID_FORMAT,
                f"output does not contain JSON object: {output}",
            )

    # escape strings with multiline content
    # these aren't technically valid JSON, but they're very useful for model output
    def sub_multiline_str(match):
        # replace line breaks with \n escape sequence
        modified_string = match.group(1).replace("\n", "\\n").replace("\r", "")
        return f'"{modified_string}"'

    value = re.compile(r'"(.*?)(?<!\\)"', re.DOTALL).sub(sub_multiline_str, value)

    try:
        ret = json.loads(value)
        ret = map_value(
            ret,
            type,
            map_v=instantiate_py_value_flat,
            is_output=True,
            ignore_outer_map=True,
        )
        check_type(ret, type, is_output=True)
        ret = DotDict(**ret)  # behave like a typed dict
        return ret
    except Exception as e:
        if isinstance(e, JSONDecodeError):
            error_type = TaskErrorType.INVALID_FORMAT
        elif isinstance(e, TypeError):
            error_type = TaskErrorType.INVALID_TYPE
        else:
            error_type = TaskErrorType.UNKNOWN
        raise TaskError(
            type=error_type, message=f"output is invalid for {type}: {e}", path=None
        ) from e
