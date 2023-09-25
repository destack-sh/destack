import abc
import asyncio
import enum
import random
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Self, Union

import structlog

from bench.language.const import IssueType
from bench.language.field import TypedDict
from bench.language.mapping import check_type, unpack_value
from bench.language.model import HasModel
from bench.language.module import ModuleNode, Scope, node_component, nruntime
from bench.language.reference import ModuleView
from ..utils.func import describe_type

if TYPE_CHECKING:
    from bench.language import HasRun, Model, Run, Statement

logger = structlog.get_logger(__name__)


@node_component(dynamic=True)
class HasTask(ModuleNode):
    _root_models: list["HasModel"] | None = nruntime(default=None)
    _randomize: bool = nruntime(default=False)

    @property
    def _is_async(self):
        return True

    def _clear_inner(self) -> None:
        self._root_models = None
        self._randomize = False

    def _interp_inner(self, scope: Scope) -> None:
        from bench.language.builtin import symbolx_lib

        randomize_tag = symbolx_lib.lookup_or_error(".builtins.randomize")
        self._randomize = self.has_tag(randomize_tag)

        if not self.outputs:
            self._on_issue(subject=self, type=IssueType.TASK_MISSING_IO)
        # TODO @UX @Task: interp task feasibility
        #  - check if task is possible given the fields, models & available runnables

    async def __call_async__(
        self,
        *args,
        _retries: int = None,
        _cache: bool = None,
        _timeout: float = None,
        _randomize: bool = None,
        _nonce: str = None,
        **kwargs,
    ):
        inputs = self._inputs_from_args(args, kwargs)

        # shortcut for built-in tasks with fixed implementations
        if self.path == "symbolx.lib.builtins.embed":
            mono_model: Optional["Model"] = self.session.module.lookup_or_error(
                "openai.lib.text.ada"
            )
        elif self.path == "symbolx.lib.builtins.transcribe":
            raise NotImplementedError
        else:
            mono_model = None

        # do task
        _randomize = _randomize if _randomize is not None else self._randomize
        view = ModuleView(self.module, self)
        view.collect()
        try:
            self.session.tracer.run_enter(self, inputs)
            if mono_model:
                outputs = await mono_model(**inputs)
                # trim output to own outputs
                if isinstance(outputs, dict):
                    outputs = {k: v for k, v in outputs.items() if self.has_field(k)}
                if not isinstance(outputs, TypedDict):
                    outputs = TypedDict(self, outputs, is_output=True)
            else:
                _nonce = _nonce or (str(random.randint(0, 2**16)) if _randomize else None)
                outputs = await run_task(self, view, inputs, _nonce)
            self.session.tracer.run_exit(self, outputs)
            return outputs
        except Exception as e:
            self.session.tracer.run_exception(self, e)
            raise

    def to_async(self) -> "Self":
        return self


class TaskErrorType(enum.StrEnum):
    Incapable = "Incapable"
    Timeout = "Timeout"
    InvalidFormat = "InvalidFormat"
    InvalidType = "InvalidType"
    ExceededLimit = "ExceededLimit"
    Unavailable = "Unavailable"
    Unknown = "Unknown"


UNRECOVERABLE_ERRORS = {TaskErrorType.Incapable, TaskErrorType.Unknown}
TASK_STEP_ATTEMPTS = 5
TASK_TOTAL_ATTEMPTS = 10


async def run_task(
    task: HasTask,
    view: ModuleView,
    inputs: dict,
    nonce: Optional[str],
) -> dict:
    total_attempts = 0
    step_attempts = 0
    models = [
        task.module.lookup_or_error(m)
        for m in ("openai.lib.chat.gpt4", "openai.lib.chat.gpt3", "anthropic.lib.text.claude-2")
    ]  # in priority order
    model_idx = 0
    log = logger.bind(task=task, inputs=describe_type(inputs), nonce=nonce, models=models)

    previous_results = []
    last_error = None
    while (
        step_attempts < TASK_STEP_ATTEMPTS
        and total_attempts < TASK_TOTAL_ATTEMPTS
        and model_idx < len(models)
    ):
        task.current_run.value.retries = step_attempts
        # run task step
        model = models[model_idx]
        compiler = model.compiler  # models may share a compiler
        compiled = await compiler.compile(task, view, inputs, previous_results, nonce)
        if not compiler.can_run(model, compiled):
            model_idx += 1
            step_attempts = 0
            continue  # try next model, not an error, just not capable

        # try step
        step_attempts += 1
        total_attempts += 1
        run_capture = task.session.capture_runs()
        try:
            log.debug("task.run", model=model, compiled=compiled, attempt=step_attempts)
            run_name = f"{task.name} #{step_attempts}"
            with task.session.tracer.run.value(retry=step_attempts, nonce=nonce, name=run_name):
                step = await compiler.run(model, compiled)
            if step.runnable is not None:
                raise NotImplementedError(":TaskFunctions not supported yet")

            # done, terminate
            # unpack -> check is not ideal since it doesn't let us collect unpack errors nicely
            outputs = unpack_value(
                step.result_raw, task, is_output=True, map_k=lambda f: (f.py_ident, f.py_ident)
            )
            check_type(outputs, task, is_output=True)
            return TypedDict(task, outputs, is_output=True)
        except Exception as e:
            e = TaskError.from_exception(task, e)
            last_error = e
            logger.debug("task.error", error=e)
            if e.type in UNRECOVERABLE_ERRORS:
                model_idx += 1
            elif e.type == TaskErrorType.ExceededLimit:
                await asyncio.sleep(0.1)
            else:
                previous_results.append(e)

            run = run_capture.stop_one_or_none()
            if run is not None:
                run.value.verdict = "reject"
                run.value.verdict_reason = str(e)

            continue

    if total_attempts == 0:
        # no models
        raise TaskError(TaskErrorType.Incapable, task, "cannot solve task with given inputs")
    else:
        raise TaskError(
            TaskErrorType.ExceededLimit,
            task,
            f"could not solve task in {TASK_STEP_ATTEMPTS} attempts across {len(models)} models:\n{last_error or '<no details>'}",
        )


# avoid circular import
from .run import RunError, RunErrorKind  # noqa: E402


class TaskError(RunError):
    def __init__(
        self,
        type: TaskErrorType,
        runnable: "Statement",
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
    def from_exception(task: "Statement", e: Exception, path: str = None) -> "TaskError":
        if isinstance(e, TaskError):
            return e
        elif isinstance(e, ValueError):
            return TaskError(TaskErrorType.InvalidFormat, task, str(e), path)
        elif isinstance(e, TypeError):
            return TaskError(TaskErrorType.InvalidType, task, str(e), path)
        else:
            return TaskError(TaskErrorType.Unknown, task, str(e), path)


class IncapableError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.Incapable, message, path)


class LimitExceededError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.ExceededLimit, message, path)


@dataclass
class CompiledInput(abc.ABC):
    task: HasTask


@dataclass
class TaskOutput(abc.ABC):
    """
    Output of a basic task run.
    If runnable is given, it's a function call, otherwise it terminates."""

    result_raw: dict  # raw (i.e. not instantiated) result
    runnable: Optional["HasRun"] = None


class TaskCompiler(abc.ABC):
    async def compile(
        self,
        task: HasTask,
        view: ModuleView,
        inputs: dict,
        previous_results: list[Union[TaskError, "Run"]],
        nonce: Optional[str],
    ) -> CompiledInput:
        raise NotImplementedError

    def can_run(self, model: "Model", input: CompiledInput) -> bool:
        raise NotImplementedError

    async def run(self, model: "Model", input: CompiledInput) -> TaskOutput:
        raise NotImplementedError
