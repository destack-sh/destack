import abc
import asyncio
import dataclasses
from dataclasses import dataclass
from typing import TYPE_CHECKING, ClassVar, Iterable, dataclass_transform
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.code import Code
from bench.language.const import RunStatus
from bench.language.field import TypeInfoBase
from bench.language.log import LogInfo
from bench.language.run import Run, RunAttempt, RunError, RunKind, RunnableNode, RunOptions
from bench.language.text import Text
from bench.language.value import CustomObject

if TYPE_CHECKING:
    from bench.runtime.runtime import Runtime


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RunnerCache[T: RunnableNode]:
    """
    The cached state of some runnable unit (Node or something within).
    May persist across runs (i.e. across Runners).
    """

    id: UUID
    kind: RunKind
    node: T
    code: Code | None = None
    text: Text | None = None
    variables: CustomObject | None = None

    def __str__(self):
        return f"{self.kind.bench_name}: {self.id} (in {self.node})"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}"


@dataclass(slots=True, repr=False)
class Runner[S: RunnerCache, T: RunnableNode]:
    """A runner for a single run (tracked or untracked)."""

    cache_cls: ClassVar[type[RunnerCache]] = RunnerCache

    id: UUID  # Run.id if tracked, new otherwise
    runtime: "Runtime"
    cache: S
    options: RunOptions
    parent: "Runner | None"  # if nested
    status: RunStatus
    inputs: CustomObject | None = None
    input_type: TypeInfoBase | None = None
    outputs: CustomObject | None = None
    output_type: TypeInfoBase | None = None
    error: RunError | None = None
    attempts: list[RunAttempt] = dataclasses.field(default_factory=list)
    logs: list[LogInfo] = dataclasses.field(default_factory=list)
    task: asyncio.Task | None = None  # the active callable being run
    runs: list["Runner"] = dataclasses.field(default_factory=list)  # nested Runners
    run: Run | None = None  # if tracked
    is_cancelled: bool = False  # whether this run was cancelled before it ran

    def __str__(self):
        str_parts: list[str] = [
            self.status.bench_name,
            f"runnable={self.cache!r}",
            f"options={self.options!r}",
        ]
        if self.attempts:
            str_parts.append(f"attempts={self.attempts}")
        if self.runs:
            str_parts.append(f"runs={len(self.runs)}")
        if self.run:
            str_parts.append(f"run={self.run}")
        return ", ".join(str_parts)

    def __repr__(self):
        content_str = str(self)
        return (
            f"<{self.__class__.__name__} {content_str}>"
            if content_str
            else f"<{self.__class__.__name__}>"
        )

    def walk(self) -> Iterable["Runner"]:
        yield self
        for run in self.runs:
            yield from run.walk()

    @property
    def is_tracked(self) -> bool:
        return self.run is not None

    @property
    def is_nested(self) -> bool:
        return self.parent is not None

    @property
    def kind(self) -> RunKind:
        return self.cache.kind

    @property
    def node(self) -> T:
        return self.cache.node

    @property
    def code(self) -> Code | None:
        return self.cache.code

    @property
    def text(self) -> Text | None:
        return self.cache.text

    @property
    def variables(self) -> CustomObject | None:
        return self.cache.variables

    @property
    def current_attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None

    @abc.abstractmethod
    async def run_once(self) -> None:
        """Runs the runnable once. Assumes  all dependencies resolved."""
        raise NotImplementedError


@dataclass_transform()
def runner_():
    """Registers a Runner for a specific RunnableType."""

    def decorator(cls):
        cls = dataclass(cls, repr=False, slots=True)  # type: ignore
        return cls

    return decorator
