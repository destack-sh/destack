import base64
import contextvars
import dataclasses
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Mapping
from uuid import UUID

from bench import language
from bench.language.code import Code
from bench.language.const import BenchError, RunStatus
from bench.language.node import Node
from bench.language.run import Run, RunAttempt, RunError, RunErrorType, RunOptions
from bench.language.session import Session
from bench.language.setup import BENCH_CLASS_BY_NAME
from bench.language.text import Text
from bench.language.value import ValueObject

if TYPE_CHECKING:
    from bench.runtime.compiler import CodeCompilation

DEFAULT_CODE_RUN_OPTIONS = RunOptions(max_attempts=1)
DEFAULT_TEXT_RUN_OPTIONS = RunOptions(max_attempts=3, retry_interval=3, backoff=2)
DEFAULT_FLOW_RUN_OPTIONS = RunOptions(max_attempts=1)

# all bench types
CODE_GLOBALS: dict[str, Any] = {**vars(language), **BENCH_CLASS_BY_NAME}
# and some general stuff
for t in (datetime, timedelta, UUID, base64):
    CODE_GLOBALS[t.__name__] = t


class BenchRuntimeError(BenchError, RuntimeError):
    pass


class NotRunnableError(BenchRuntimeError):
    run_error_type = RunErrorType.NOT_RUNNABLE


class RunHaltedError(BenchRuntimeError):
    pass


@dataclass(slots=True)
class RunnableState:
    """The state of some runnable unit (Node or something within)."""

    id: UUID
    scope: Node
    code: Code | None
    text: Text | None
    compiled: "CodeCompilation | None" = None
    variables: ValueObject | None = None

    # outputs
    exports: Mapping[str, Any] | None = None  # for scripts
    last_expr_value: Any | None = None  # for snippets


@dataclass(slots=True)
class RunHandle:
    """A specific run (tracked or untracked)."""

    id: UUID  # Run.id if tracked, new otherwise
    runnable: RunnableState
    options: RunOptions
    parent: "RunHandle | None"  # if nested
    status: RunStatus
    inputs: ValueObject | None = None
    outputs: ValueObject | None = None
    error: RunError | None = None
    attempts: list[RunAttempt] = dataclasses.field(default_factory=list)
    run: Run | None = None  # if tracked

    def __str__(self):
        str_parts: list[str] = [f"runnable={self.runnable!r}", f"options={self.options!r}"]
        if self.attempts:
            str_parts.append(f"attempts={self.attempts}")
        if self.run:
            str_parts.append(f"run={self.run}")
        return ", ".join(str_parts)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}"

    @property
    def attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None


class RuntimeState:
    """
    The state of a specific runtime.
    Encapsulates the current stack, any defined exports, inter-dependencies, etc.
    We don't actually run anything here, just figure out what to run and in what order.
    """

    def __init__(self, *, session: Session, glbls: Mapping[str, Any] = CODE_GLOBALS):
        self.session = session
        self.glbls = glbls

        self._run_stack: contextvars.ContextVar[list[RunHandle]] = contextvars.ContextVar(
            "run_stack"
        )
        self._runnables: dict[UUID, RunnableState] = {}
        self._active_runs: dict[UUID, RunHandle] = {}
