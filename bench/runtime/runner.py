import abc
import asyncio
import dataclasses
from dataclasses import dataclass
from typing import TYPE_CHECKING, Iterable
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.const import RunStatus
from bench.language.field import TypeInfoBase
from bench.language.log import LogInfo
from bench.language.run import Run, RunAttempt, RunError, RunKind, RunOptions
from bench.language.value import CustomObject

if TYPE_CHECKING:
    from bench.language import Block, Step
    from bench.runtime.runtime import Runtime


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True, repr=False)
class Runner:
    """A runner for a single Run (tracked Run or untracked RunSpan)."""

    id: UUID  # Run.id if tracked, new otherwise
    kind: RunKind
    runtime: "Runtime"
    options: RunOptions
    parent: "Runner | None"  # if nested
    status: RunStatus
    node: "Block | Step"
    inputs: CustomObject | None = None
    input_type: TypeInfoBase | None = None
    outputs: CustomObject | None = None
    output_type: TypeInfoBase | None = None
    variable_type: TypeInfoBase | None = None
    variables: CustomObject | None = None
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
            f"node={self.node!r}",
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
    def current_attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None

    @abc.abstractmethod
    async def run_once(self) -> None:
        """Runs the runnable once."""
        raise NotImplementedError