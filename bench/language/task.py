import abc
import enum
import itertools
import random
from dataclasses import dataclass
from typing import Collection, Optional, Self, Union

from bench.language.basic import HasText
from bench.language.const import StatementType, TypeTag
from bench.language.core import Module, ModuleNode, ModuleVisitor, Scope, Statement, node
from bench.language.flow import HasFlow, IsFlowNode
from bench.language.model import Model
from bench.language.reflect import reflect_struct
from bench.language.session import Run, RunError, RunErrorKind
from bench.language.tag import HasTags
from bench.language.type import HasType, instantiate_value
from bench.language.utils import Runnable
from bench.utils.utils import DotList


class TaskErrorType(enum.StrEnum):
    Incapable = "Incapable"
    Timeout = "Timeout"
    InvalidFormat = "InvalidFormat"
    InvalidType = "InvalidType"
    TooLarge = "TooLarge"
    ExceededLimit = "ExceededLimit"
    Unknown = "Unknown"


class TaskError(RunError):
    def __init__(
        self,
        type: TaskErrorType,
        runnable: Union[Model, "Task"],
        message: str = None,
        path: str = None,
    ):
        super().__init__(
            kind=RunErrorKind.Runtime,
            type=type.name,
            runnable=runnable,
            message=f"{type.value}: {message}",
        )
        self.type = type
        self.path = path

    @staticmethod
    def from_exception(e: Exception, path: str = None) -> "TaskError":
        if isinstance(e, TaskError):
            return e
        elif isinstance(e, ValueError):
            return TaskError(TaskErrorType.InvalidFormat, str(e), path)
        elif isinstance(e, TypeError):
            return TaskError(TaskErrorType.InvalidType, str(e), path)
        else:
            return TaskError(TaskErrorType.Unknown, str(e), path)


class IncapableError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.Incapable, message, path)


class LimitExceededError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.ExceededLimit, message, path)


@reflect_struct("TaskMetadata", "Default metadata of a task", return_type=True)
class TaskMetadata:
    retries: Optional[int]
    retry: Optional[int]
    batch_size: Optional[int]
    nonce: Optional[str]


@node(tracked=["text"])
class Task(HasType, HasFlow, IsFlowNode, HasTags, HasText, Runnable, Statement):
    tag: TypeTag = TypeTag.FUNCTION
    type: StatementType = StatementType.TASK
    _is_async: bool = True
    _root_models: list[Model] = None
    _randomized: bool = False

    def _visit(self, visitor: "ModuleVisitor") -> None:
        for n in itertools.chain(self.fields, self.tags, self.triggers):
            visitor.visit_child(n)

    def _clear(self) -> None:
        Statement._clear(self)
        HasType._clear(self)
        HasTags._clear(self)
        IsFlowNode._clear(self)

    def _interp(self, scope: Scope) -> None:
        from bench.language.builtin import symbolx_lib

        HasType._interp(self, scope)
        HasTags._interp(self, scope)
        IsFlowNode._interp(self, scope)

        randomize_tag = symbolx_lib.lookup_or_error(".builtins.randomize")
        self._randomized = self.has_tag(randomize_tag)

        # TODO @Broken @UX: interp task
        #  - check if task is possible given the fields, models & available runnables

    async def __call__(
        self,
        *args,
        retries: int = None,
        cache: bool = None,
        timeout: float = None,
        batch: list[dict] = None,
        **kwargs,
    ):
        # map/batch inputs
        inputs = self._inputs_from_args(args, kwargs)
        is_batched = batch is not None
        if is_batched and not isinstance(batch, DotList):
            inputs = DotList(batch)
            del batch

        # shortcut for built-in tasks with fixed implementations
        if self.path == "symbolx.lib.builtins.embed":
            mono_model: Model | None = self.module.lookup_or_error("openai.lib.text.ada")
        elif self.path == "symbolx.lib.builtins.transcribe":
            raise NotImplementedError
        else:
            root_models = [self.module.lookup_or_error("openai.lib.chat.gpt4")]
            mono_model = None

        # do task
        view = ModuleView(self.module, self)
        view.collect(max_child_depth=None, max_reference_depth=1)
        try:
            self.session.tracer.run_enter(self, inputs)
            if inputs and is_batched:
                raise ValueError("cannot specify both inputs and batch")

            if mono_model:
                output = await mono_model()
            else:
                nonce = str(random.randint(0, 2**16)) if self._randomized else None
                output = await run_task(self, root_models, view, inputs, nonce)
            self.session.tracer.run_exit(self, output)
            return output
        except Exception as e:
            self.session.tracer.run_exception(self, e)
            raise

    def to_async(self) -> "Self":
        return self

    def to_sync(self) -> "Self":
        if self._is_async:
            return TaskProxy.to_sync(self)
        return self


class ModuleView(ModuleVisitor):
    def __init__(self, module: Module, origin: Statement):
        super().__init__()
        self.module = module
        self.origin = origin

    @property
    def nodes(self) -> Collection[ModuleNode]:
        return self._visited_node_by_ck.values()

    def collect(self, max_child_depth: Optional[int], max_reference_depth: Optional[int]):
        visitor = ModuleVisitor()
        visitor.visit_child(self.origin)
        pass  # nocheckin expand view properly


async def run_task(
    task: Task, root_models: list[Model], view: ModuleView, inputs: dict, nonce: Optional[str]
):
    num_retries_total = 0
    root_model = root_models[0]

    while num_retries_total < 5:
        # nocheckin: track metadata in task/model runs
        previous_results = []

        # get next thing to run or terminate
        compiled = root_model.compiler.compile(task, view, inputs, previous_results, nonce)
        try:
            output = await root_model.compiler.run(root_model, compiled)
            if output.runnable is None:
                # done, terminate
                return instantiate_value(output.result_raw, task, is_output=True)

            # runnable to call
            inputs = instantiate_value(output.result_raw, output.runnable, is_output=False)
        except Exception as e:
            previous_results.append(TaskError.from_exception(e))
            continue

        # call runnable
        try:
            _ = await output.runnable(**inputs)
        except Exception:  # noqa: E722
            pass  # nothing to do, already captured by tracer

        # nocheckin: get run from tracer somehow?


@dataclass
class CompiledInput(abc.ABC):
    task: Task


@dataclass
class TaskOutput(abc.ABC):
    """
    Output of a basic task run.
    If runnable is given, it's a function call, otherwise it terminates."""

    result_raw: dict  # raw (i.e. not instantiated) result
    runnable: Optional[Runnable] = None


class TaskCompiler(abc.ABC):
    def compile(
        self,
        task: Task,
        view: ModuleView,
        inputs: dict,
        previous_results: list[TaskError | Run],
        nonce: Optional[str],
    ) -> CompiledInput:
        raise NotImplementedError

    async def run(self, model: Model, input: CompiledInput) -> TaskOutput:
        raise NotImplementedError


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
