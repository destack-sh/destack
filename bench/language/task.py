import abc
import enum
import random
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Self, Union

from more_itertools import first

from bench.language.const import IssueType
from bench.language.field import HasFields
from bench.language.mapping import check_type, unpack_value
from bench.language.model import HasModel
from bench.language.module import ModuleNode, ModuleVisitor, Scope, node
from bench.language.reference import ModuleView
from bench.utils.utils import DotDict

if TYPE_CHECKING:
    from bench.language import HasRun, Model, Run, Statement


@node
class HasTask(HasFields, ModuleNode):
    _is_async: bool = True
    _root_models: list["HasModel"] = None
    _randomize: bool = False

    def _clear(self) -> None:
        pass

    def _index(self) -> None:
        pass

    def _interp(self, scope: Scope) -> None:
        from bench.language.builtin import symbolx_lib

        randomize_tag = symbolx_lib.lookup_or_error(".builtins.randomize")
        self._randomize = self.has_tag(randomize_tag)

        if not self.outputs:
            self._on_issue(subject=self, type=IssueType.TASK_MISSING_IO)
        # TODO @UX @Task: interp task
        #  - check if task is possible given the fields, models & available runnables

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass

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
            root_models = [self.session.module.lookup_or_error("openai.lib.chat.gpt4")]
            mono_model = None

        # do task
        _randomize = _randomize if _randomize is not None else self._randomize
        view = ModuleView(self.module, self)
        view.collect()
        try:
            self.session.tracer.run_enter(self, inputs)
            if mono_model:
                output = await mono_model(**inputs)
                # trim output to own outputs
                if isinstance(output, dict):
                    output = {k: v for k, v in output.items() if self.has_field(k)}
                if not isinstance(output, DotDict):
                    output = DotDict(output)
            else:
                _nonce = _nonce or (str(random.randint(0, 2**16)) if _randomize else None)
                output = await run_task(self, root_models, view, inputs, _nonce)
            self.session.tracer.run_exit(self, output)
            return output
        except Exception as e:
            self.session.tracer.run_exception(self, e)
            raise

    def to_async(self) -> "Self":
        return self


async def run_task(
    task: HasTask,
    root_models: list["HasModel"],
    view: ModuleView,
    inputs: dict,
    nonce: Optional[str],
) -> dict:
    num_retries_total = 0
    root_model = root_models[0]  # TODO @Broken @Tass: auto-select between multiple root models

    previous_results = []
    while num_retries_total < 5:
        num_retries_total += 1
        # TODO @UX @Task: track metadata in task/model runs
        # get next thing to run or terminate
        compiled = await root_model.compiler.compile(task, view, inputs, previous_results, nonce)
        try:
            step = await root_model.compiler.run(root_model, compiled)
            if step.runnable is None:
                # done, terminate
                # unpack -> check is not ideal since it doesn't let us collect unpack errors nicely
                output = unpack_value(
                    step.result_raw, task, is_output=True, map_k=lambda f: (f.py_ident, f.py_ident)
                )
                check_type(output, task, is_output=True)
                return DotDict(output)

            # runnable to call
            inputs = unpack_value(step.result_raw, step.runnable, is_output=False)
        except Exception as e:
            previous_results.append(TaskError.from_exception(e))
            continue

        # call runnable
        try:
            with task.session.tracer.run.capture() as capture:
                _ = await step.runnable(**inputs)
            run = first((r for r in capture.runs if r.runnable == step.runnable), None)
            assert run is not None, f"runnable run not found: {step.runnable}"
        except Exception:  # noqa
            pass  # nothing to do, already captured by tracer

    raise TaskError(TaskErrorType.ExceededLimit, task, f"max retries exceeded: {num_retries_total}")


class TaskErrorType(enum.StrEnum):
    Incapable = "Incapable"
    Timeout = "Timeout"
    InvalidFormat = "InvalidFormat"
    InvalidType = "InvalidType"
    TooLarge = "TooLarge"
    ExceededLimit = "ExceededLimit"
    Unknown = "Unknown"


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

    async def run(self, model: "Model", input: CompiledInput) -> TaskOutput:
        raise NotImplementedError
