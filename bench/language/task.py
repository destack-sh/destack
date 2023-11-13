import abc
import asyncio
import enum
import random
from dataclasses import dataclass
from typing import TYPE_CHECKING, Literal, Optional, Union

import structlog

from bench.language.const import MNT, IssueType, TypeFlag
from bench.language.field import TypedDict
from bench.language.model import HasModel
from bench.language.module import Node, ScopeNode, node_component, nruntime
from bench.language.reference import NodeView

from ..utils.func import describe_type

if TYPE_CHECKING:
    from bench.language import Run, Statement

logger = structlog.get_logger(__name__)


@node_component
class HasTask(Node):
    _root_models: list["HasModel"] | None = nruntime(default=None)
    _randomize: bool = nruntime(default=False)

    @property
    def _is_async(self):
        return True

    def _clear_inner(self, scope: Optional[ScopeNode]) -> None:
        self._root_models = None
        self._randomize = False

    def _interp_inner(self, scope: ScopeNode) -> None:
        from bench.language.builtin import symbolx_lib

        randomize_tag = symbolx_lib.resolve(".builtins.randomize")
        self._randomize = randomize_tag in self.tags

        if not any(f.flags & TypeFlag.IS_OUTPUT for f in self.resolved_fields):
            self._on_issue(subject=self, type=IssueType.TASK_MISSING_IO)
        # TODO @UX @Task: interp task feasibility
        #  - check if task is possible given the fields, models & available statements

    async def _call_inner_async(
        self,
        *args,
        # :TaskConfig
        cache: bool = None,
        nonce: str = None,
        mode: Literal["fast", "deliberate", "auto"] = "auto",
        **kwargs,
    ):
        # prepare inputs
        nonce = nonce or (str(random.randint(0, 2**16)) if self._randomize else None)
        kwargs["cache"] = cache
        kwargs["nonce"] = nonce
        kwargs["mode"] = mode
        inputs = self._inputs_from_args(args, kwargs)

        # shortcut for built-in tasks with fixed implementations
        if self.path == "symbolx.lib.builtins.embed":
            passthrough_model: "Statement" = self.session.module.resolve("openai.lib.text.ada")
        elif self.path == "symbolx.lib.builtins.transcribe":
            passthrough_model: "Statement" = self.session.module.resolve(
                "deepgram.lib.audio.nova-2"
            )
            if inputs.get("file"):  # replace file with url to blob
                inputs["url"] = await inputs["file"].aget_url()
                del inputs["file"]
        else:
            passthrough_model = None
            if mode == "auto":
                mode = "fast"
            if mode == "fast":
                models = ["anthropic.lib.text.claude-instant-1", "openai.lib.chat.gpt3-turbo"]
            else:
                models = ["openai.lib.chat.gpt4-turbo", "anthropic.lib.text.claude-2"]
            models = [self.session.module.resolve(m) for m in models]

        # do task
        view = NodeView(self.module)
        seen_from_node = view.view_node(self, ancestors_up_to=MNT.FILE, max_distance=5)
        await view.view_records(seen_from_node.values(), limit=10)
        seen_from_value = view.view_value(inputs, self, is_output=False)
        view.view_node(seen_from_value.values(), ancestors_up_to=MNT.FILE, max_distance=2)
        await view.view_records(seen_from_value.values(), limit=10)

        self.session._tracer.run_enter(self, is_async=True, inputs=inputs)
        try:
            if passthrough_model:
                # passthrough model
                inputs = {k: v for k, v in inputs.items() if k not in ("cache", "nonce", "mode")}
                outputs = await passthrough_model(**inputs)
                # trim output to own outputs
                if isinstance(outputs, dict):
                    outputs = {k: v for k, v in outputs.items() if k in self.fields}
                if not isinstance(outputs, TypedDict):
                    outputs = TypedDict(outputs, self, is_output=True)
            else:
                outputs = await run_task(self, view, inputs, nonce, models)
            if self.session._autocommit_this_run:
                await self.session.acommit()
        except BaseException as e:
            self.session._tracer.run_exception(self, e)
            raise
        self.session._tracer.run_exit(self, outputs)
        return outputs


class TaskErrorType(enum.StrEnum):
    Incapable = "Incapable"
    Timeout = "Timeout"
    InvalidFormat = "InvalidFormat"
    InvalidType = "InvalidType"
    ExceededLimit = "ExceededLimit"
    Unavailable = "Unavailable"
    Unknown = "Unknown"


UNRECOVERABLE_ERRORS = {TaskErrorType.Incapable, TaskErrorType.Unknown}
TASK_MODEL_ATTEMPTS = 3
TASK_TOTAL_ATTEMPTS = 10


async def run_task(
    task: HasTask,
    view: NodeView,
    inputs: dict,
    nonce: Optional[str],
    models: list["Statement"] = None,
) -> dict:
    from bench.language.packer import check_type, unpack_value

    total_attempts = 0
    model_attempts = 0
    # in priority order
    model_idx = 0
    log = logger.bind(task=task, inputs=describe_type(inputs), nonce=nonce, models=models)

    previous_results = []
    last_error = None
    while (
        model_attempts < TASK_MODEL_ATTEMPTS
        and total_attempts < TASK_TOTAL_ATTEMPTS
        and model_idx < len(models)
    ):
        task.current_run.value.retries = model_attempts
        # run task step
        model = models[model_idx]
        compiler = model.compiler  # models may share a compiler
        compiled = await compiler.compile(task, view, inputs, previous_results, nonce)
        if not compiler.can_run(model, compiled):
            model_idx += 1
            model_attempts = 0
            continue  # try next model, not an error, just not capable

        # try model
        model_attempts += 1
        total_attempts += 1
        run_capture = task.session.capture_runs()
        try:
            log.debug("task.run", model=model, compiled=compiled, attempt=model_attempts)
            run_name = f"{task.name} #{total_attempts}"
            with task.session.bind_run_value(retry=model_attempts, nonce=nonce, name=run_name):
                outputs = await compiler.run(model, compiled)

            # done, terminate
            # unpack -> check is not ideal since it doesn't let us collect unpack errors nicely
            outputs = unpack_value(
                outputs, task, is_output=True, map_k=lambda f: (f.py_ident, f.py_ident)
            )
            check_type(outputs, task, is_output=True)
            return TypedDict(outputs, task, is_output=True)
        except Exception as e:
            e = TaskError.from_exception(task, e)
            last_error = e
            logger.debug("task.error", error=e)
            if e.type in UNRECOVERABLE_ERRORS:
                model_idx += 1
                model_attempts = 0
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
        raise TaskError(TaskErrorType.Incapable, task, "cannot solve task with given input types")
    else:
        raise TaskError(
            TaskErrorType.ExceededLimit,
            task,
            f"could not solve task in {TASK_MODEL_ATTEMPTS} attempts across {len(models)} models:\n{last_error or '<no details>'}",
        )


# avoid circular import
from .run import RunError, RunErrorKind  # noqa: E402


class TaskError(RunError):
    def __init__(
        self,
        type: TaskErrorType,
        statement: "Statement",
        message: str = None,
        path: str = None,
    ):
        super().__init__(
            kind=RunErrorKind.Runtime,
            type=type.name,
            statement=statement,
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


class TaskCompiler(abc.ABC):
    async def compile(
        self,
        task: HasTask,
        view: NodeView,
        inputs: dict,
        previous_results: list[Union[TaskError, "Run"]],
        nonce: Optional[str],
    ) -> CompiledInput:
        raise NotImplementedError

    def can_run(self, model: "Statement", input: CompiledInput) -> bool:
        raise NotImplementedError

    async def run(self, model: "Statement", input: CompiledInput) -> dict:
        raise NotImplementedError
