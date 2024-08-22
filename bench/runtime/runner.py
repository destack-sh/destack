import abc
import asyncio
import dataclasses
from dataclasses import dataclass
from typing import ClassVar, dataclass_transform
from uuid import UUID

import structlog
from git import TYPE_CHECKING
from opentelemetry import trace

from bench.language.block import Block
from bench.language.code import Code, CodeType
from bench.language.const import RunStatus
from bench.language.field import TypeInfoBase
from bench.language.log import LogInfo
from bench.language.run import ModelProvider, Run, RunAttempt, RunError, RunKind, RunOptions
from bench.language.step import Step, StepType
from bench.language.text import Text
from bench.language.value import ValueObject

if TYPE_CHECKING:
    from bench.runtime.runtime import Runtime


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

RunnableNode = Block | Step
RunSubtype = CodeType | StepType | ModelProvider


@dataclass(slots=True)
class RunnerCache[T: RunnableNode]:
    """
    The cached state of some runnable unit (Node or something within).
    May persist across runs (i.e. across Runners).
    """

    id: UUID
    kind: RunKind
    subtype: RunSubtype | None
    node: T
    code: Code | None = None
    text: Text | None = None
    variables: ValueObject | None = None

    def __str__(self):
        type_str = (
            f"{self.kind.bench_name}:{self.subtype.bench_name}"
            if self.subtype
            else self.kind.bench_name
        )
        return f"{type_str}: {self.id} (in {self.node})"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}"


@dataclass(slots=True)
class Runner[S: RunnerCache, T: RunnableNode](abc.ABC):
    """A runner for a single run (tracked or untracked)."""

    cache_cls: ClassVar[type[RunnerCache]] = RunnerCache

    id: UUID  # Run.id if tracked, new otherwise
    runtime: "Runtime"
    cache: S
    options: RunOptions
    parent: "Runner | None"  # if nested
    status: RunStatus
    inputs: ValueObject | None = None
    input_type: TypeInfoBase | None = None
    outputs: ValueObject | None = None
    output_type: TypeInfoBase | None = None
    error: RunError | None = None
    attempts: list[RunAttempt] = dataclasses.field(default_factory=list)
    logs: list[LogInfo] = dataclasses.field(default_factory=list)
    inner_task: asyncio.Task | None = None  # the active callable being run
    runs: list["Runner"] = dataclasses.field(default_factory=list)  # nested Runners
    run: Run | None = None  # if tracked

    def __str__(self):
        str_parts: list[str] = [f"runnable={self.cache!r}", f"options={self.options!r}"]
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
    def variables(self) -> ValueObject | None:
        return self.cache.variables

    @property
    def key(self) -> RunSubtype | None:
        return self.cache.subtype

    @property
    def current_attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None

    @abc.abstractmethod
    async def run_once(self) -> None:
        """Runs the runnable once. Assumes  all dependencies resolved."""
        raise NotImplementedError


_runners: dict[tuple[RunKind, RunSubtype | None], type[Runner]] = {}


@dataclass_transform()
def runner_(kind: RunKind, typ: RunSubtype | None = None):
    """Registers a Runner for a specific RunnableType."""

    def decorator(cls):
        if (kind, typ) in _runners:
            raise ValueError(f"runner already registered for {typ}: {_runners[(kind, typ)]!r}")
        cls = dataclass(cls, repr=False, slots=True)  # type: ignore
        _runners[(kind, typ)] = cls
        return cls

    return decorator
