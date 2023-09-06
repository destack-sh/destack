import enum
import itertools
import random
from typing import Optional, Self

from bench.language.basic import HasText
from bench.language.const import StatementType, TypeTag
from bench.language.core import ModuleVisitor, Scope, Statement, node
from bench.language.flow import HasFlow, IsFlowNode
from bench.language.model import Model
from bench.language.reflect import reflect_struct
from bench.language.tag import HasTags
from bench.language.type import HasType
from bench.language.utils import Runnable
from bench.utils.utils import DotList


class TaskErrorType(enum.StrEnum):
    INCAPABLE = "incapable"
    TIMEOUT = "timeout"
    INVALID_FORMAT = "invalid_format"
    INVALID_TYPE = "invalid_type"
    EXCEEDED_LIMIT = "exceeded_limit"
    UNKNOWN = "unknown"


class TaskError(ValueError):
    def __init__(self, type: TaskErrorType, message: str = None, path: str = None):
        super().__init__(f"{type.value}: {message}")
        self.type = type
        self.path = path

    @staticmethod
    def from_exception(e: Exception, path: str = None) -> "TaskError":
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


@reflect_struct("TaskMetadata", "Default metadata of a task", return_type=True)
class TaskMetadata:
    retries: Optional[int]
    nonce: Optional[str]


@node(tracked=["text"])
class Task(HasType, HasFlow, IsFlowNode, HasTags, HasText, Runnable, Statement):
    tag: TypeTag = TypeTag.FUNCTION
    type: StatementType = StatementType.TASK
    _is_async: bool = True

    def _clear(self) -> None:
        Statement._clear(self)
        HasType._clear(self)
        HasTags._clear(self)
        IsFlowNode._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        HasTags._interp(self, scope)
        IsFlowNode._interp(self, scope)

    def _visit(self, visitor: "ModuleVisitor") -> None:
        for n in itertools.chain(self.fields, self.tags, self.triggers):
            visitor.visit(n)

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
        from bench.language.builtin import symbolx_lib

        # map/batch inputs
        inputs = self._inputs_from_args(args, kwargs)
        is_batched = batch is not None
        if is_batched and not isinstance(batch, DotList):
            inputs = DotList(batch)
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

            # nocheckin: simple task compilation
            # - decision model
            #   - function calling
            #     - tool 'coercion' (e.g. dataset -> metadata + search function)
            # - tool models == functions? (but with more or less flexible I/O)
            # - has flow (NOT YET)
            #   - constrained sub-flows
            #   - triggers
            #   - 'inlined' pre/post code (e.g. to include context based on query)
            #   - interrupts & reproducibility
            #    - nonces (put into metadata?)
            # - task metadata
            #   - progress reporting?
            #   - retries
            # - flexible rendering
            #   - 'compilation'/rendering for (annotated) text, types, tools, etc.
            #   - walk statements?
            # - automatic model selection
            # - error handling
            #   - automatic retries & fallbacks
            # - interp (warnings/errors)
            #  - should use same 'parsing' logic in interp (maybe even pre-parse?)
            # - continuous granularity
            #  - plug in different flow runner?

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
                randomize_tag = symbolx_lib.lookup_or_error(".builtins.randomize")
                if self.has_tag(randomize_tag):
                    nonce = str(random.randint(0, 2**16))
                else:
                    nonce = None

                runner = TaskRunner(
                    self,
                    max_steps=20,
                    max_function_calls=10,
                    max_errors=5,
                    max_model_errors=5,
                    nonce=nonce,
                )
                ret = await runner(model[0], inputs, is_batched)
            self.session.tracer.run_exit(self, ret)
            return ret
        except Exception as e:
            self.session.tracer.run_exception(self, e)
            raise

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
