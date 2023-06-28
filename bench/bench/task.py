from __future__ import annotations

import abc
import enum
import json
import re
import typing
import uuid
from json import JSONDecodeError
from typing import Optional, Self

from bench.bench import Code
from bench.bench.const import TypeFlag, TypeHint, TypeTag
from bench.bench.core import Scope, Statement, node
from bench.bench.expect import Expectation, HasExpectations
from bench.bench.model import Model
from bench.bench.type import (
    HasType,
    Type,
    TypeBase,
    check_type,
    instantiate_py_value_flat,
    map_value,
)
from bench.utils.utils import DotDict


@node(tracked=["description"])
class Task(Statement, HasType, HasExpectations):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.FUNCTION
    _is_async: bool = True

    def _clear(self) -> None:
        Statement._clear(self)
        HasType._clear(self)
        HasExpectations._clear(self)
        self._implementations = None

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
        **kwargs,
    ):
        inputs = {**kwargs}
        for input_t, input in zip(self.inputs, args):
            inputs[input_t.name] = input
        # shortcut for built-in tasks with fixed implementations
        if self.fqn == "symbolx.lib.builtins.embed":
            from bench.bench.libs import openai_lib

            ada = openai_lib.lookup("openai.lib.text.ada", statement_t=Model)
            if ada is None:
                raise RuntimeError("openai.lib.text.ada not found")
            return await ada(**inputs, retries=retries, cache=cache, timeout=timeout)
        elif self.fqn == "symbolx.lib.builtins.transcribe":
            raise NotImplementedError
        else:
            raise NotImplementedError  # nocheckin

    def to_sync(self) -> "Self":
        if self._is_async:
            return TaskProxy.to_sync(self)
        return self


class TaskProxy:
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

    @classmethod
    def to_sync(cls, task: Task) -> "TaskProxy":
        return cls(task, is_async=False)


class TaskCompiler(abc.ABC):
    def set_type(self, type: Type) -> None:
        pass

    def add_step(self, step: Task) -> None:
        pass

    def add_tool(self, tool: Task | Code | Model) -> None:
        pass

    def add_expectation(self, expectation: Expectation) -> None:
        pass

    def add_error(self, error: TaskError) -> None:
        pass


class TaskErrorType(enum.StrEnum):
    INCAPABLE = "incapable"
    TIMEOUT = "timeout"
    INVALID_FORMAT = "invalid_format"
    INVALID_TYPE = "invalid_type"
    UNKNOWN = "unknown"


class TaskError(ValueError):
    def __init__(self, type: TaskErrorType, message: str = None, path: str = None):
        super().__init__(message)
        self.type = type
        self.path = path


class IncapableError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.INCAPABLE, message, path)


async def do_task(self: Task, model: Model | str = None, retries: int = None, **kwargs):
    # get candidate task implementations
    if model is not None:
        if isinstance(model, str):
            model = self.module.lookup(model, statement_t=Model)
        models = [model]
    else:
        models = self.session.default_models
    candidates = []
    for model in models:
        cache_key = (self.id, model.id)
        if cache_key not in self._cached_implementations:
            implementation = build_task_implementation(self, model, self.session)
            self._cached_implementations[cache_key] = implementation
        candidates.append(self._cached_implementations[cache_key])
    # TODO @Broken: sort/filter implementations with some smartness
    impl_idx = self._last_good_impl_idx
    retries = retries if retries is not None else self.session.inference_retries
    remaining_retries = retries

    # actually run the task
    self.session.tracer.code_enter(self, args, kwargs)
    semantic_errors = []
    while remaining_retries >= 0:
        remaining_retries -= 1
        impl = candidates[impl_idx]
        log = self.session.logger.bind(task=self, retries=remaining_retries, implementation=impl)
        try:
            ret = await impl(*args, **kwargs, cache=cache, timeout=timeout)
            self._last_good_impl_idx = impl_idx
            self.session.tracer.code_exit(self, args, kwargs, ret)
            return ret
        except TaskError as e:
            semantic_errors.append(e)
            log.warning("task.failed", exc_info=e)
            if len(semantic_errors) <= self.session.inference_retries / len(candidates):
                # retry with error info a few times
                candidates[impl_idx] = impl.copy().emit(XConsiderError(e))
            else:
                # fail over
                impl_idx = (impl_idx + 1) % len(candidates)
                semantic_errors = []
        except TimeoutError as e:
            # fail over
            self.session.logger.warning("task.failed", exc_info=e)
            impl_idx = (impl_idx + 1) % len(candidates)

    # give up
    errors_repr = "\n".join(str(e) for e in semantic_errors) if semantic_errors else "<timeout>"
    e = RuntimeError(f"{self} failed after {retries} retries: {errors_repr}")
    self.session.tracer.code_exception(self, args, kwargs, e)
    if semantic_errors:
        raise e from semantic_errors[-1]
    else:
        raise e


def parse_string_output(output: str, type: Type):
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


SAMPLE_BY_TYPE_HINT = {
    TypeHint.UUID: str(uuid.uuid4()),
    TypeHint.NAME: "Max Mustermann",
    TypeHint.EMAIL: "florian@symbolx.com",
    TypeHint.PHONE: "+49 123 456 789",
    TypeHint.URL: "https://symbolx.com",
    TypeHint.KEY: "sk_test_1234567890",
    TypeHint.DATE: "2023-01-01",
    TypeHint.DATETIME: "2023-01-01T10:30:45",
    TypeHint.TIME: "02:08:00",
    TypeHint.RATING: 3,
}


def fabricate_value(type: TypeBase, skip_array: bool = False, is_output: bool = None) -> typing.Any:
    """Synthesizes a value of the given type with fake fields."""
    if type.flags & TypeFlag.IsArray and not skip_array:
        return [fabricate_value(type, skip_array=True)]
    if SAMPLE_BY_TYPE_HINT.get(type.hint) is not None:
        return SAMPLE_BY_TYPE_HINT[type.hint]
    elif type.tag == TypeTag.STRING:
        return "lorem ipsum"
    elif type.tag == TypeTag.NUMBER:
        return 42
    elif type.tag == TypeTag.BOOLEAN:
        return False
    elif type.tag == TypeTag.ENUM:
        if len(type.fields) == 0:
            return None
        return type.fields[0].name
    elif type.tag == TypeTag.STRUCT or type.tag == TypeTag.FUNCTION:
        return {
            subtype.name: fabricate_value(subtype)
            for subtype in type.fields
            if is_output is None or bool(subtype.flags & TypeFlag.IsOutput) == is_output
        }
    elif type.tag == TypeTag.UNION:
        return fabricate_value(type.fields[0])
    elif type.tag == TypeTag.NULL:
        return None
    elif type.tag == TypeTag.LITERAL:
        return type.name  # assumes enum string literals
    elif type.tag == TypeTag.ANY:
        return 42  # not sure what to do here
    else:
        raise RuntimeError(f"unexpected type {type.tag}")
