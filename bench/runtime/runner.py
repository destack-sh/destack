import abc
import dataclasses
from dataclasses import dataclass
from typing import Any, Mapping
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.block import Block
from bench.language.code import Code
from bench.language.const import BenchError, BlockType, FieldZone, RunErrorKind, RunKind, RunStatus
from bench.language.node import Node
from bench.language.run import CodeKind, Run, RunAttempt, RunError, RunnableKind, RunOptions
from bench.language.session import Session
from bench.language.step import Step, StepType
from bench.language.text import Text
from bench.language.value import ValueObject
from bench.runtime.compiler import CompiledCode
from bench.runtime.core import (
    CODE_GLOBALS,
    DEFAULT_CODE_RUN_OPTIONS,
    NotRunnableError,
    RunHaltedError,
)
from bench.utils.oracle import Oracle

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

RunnableType = tuple[RunnableKind, CodeKind | StepType | None]


@dataclass(slots=True)
class RunnableState[T: Node]:
    """The state of some runnable unit (Node or something within)."""

    id: UUID
    typ: RunnableType
    node: T
    code: Code | None
    text: Text | None
    compiled: "CompiledCode | None" = None
    variables: ValueObject | None = None

    # outputs
    exports: Mapping[str, Any] | None = None  # for scripts
    last_expr_value: Any | None = None  # for snippets

    def __str__(self):
        typ_str = (
            f"{self.typ[0].bench_name}->{self.typ[1].bench_name}"
            if self.typ[1]
            else self.typ[0].bench_name
        )
        return f"{typ_str}: {self.id} (in {self.node})"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}"


@dataclass(slots=True)
class RunHandle[T: Node]:
    """A specific run (tracked or untracked)."""

    id: UUID  # Run.id if tracked, new otherwise
    runnable: RunnableState[T]
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
    def current_attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None


class Runner[T: Node](abc.ABC):
    """A runner to process a single runnable unit once."""

    __slots__ = ("handle", "runnable", "runner", "session")

    def __init__(self, runner: "RuntimeRunner", session: Session, handle: RunHandle[T]):
        self.runner = runner
        self.session = session
        self.handle = handle
        self.runnable = handle.runnable

    def __str__(self):
        return f"{self.handle!r}"

    def __repr__(self):
        content_str = str(self)
        return (
            f"<{self.__class__.__name__} {content_str}>"
            if content_str
            else f"<{self.__class__.__name__}>"
        )

    @property
    def node(self) -> T:
        return self.runnable.node

    @property
    def code(self) -> Code | None:
        return self.runnable.code

    @property
    def text(self) -> Text | None:
        return self.runnable.text

    @property
    def variables(self) -> ValueObject | None:
        return self.runnable.variables

    @property
    def status(self) -> RunStatus:
        return self.handle.status

    @property
    def inputs(self) -> ValueObject | None:
        return self.handle.inputs

    @property
    def attempts(self) -> list[RunAttempt]:
        return self.handle.attempts

    @abc.abstractmethod
    async def run(self) -> None:
        """Runs the runnable once with all dependencies resolved"""
        raise NotImplementedError


_runners: dict[RunnableType, type[Runner]] = {}


def runner(typ: RunnableType):
    """Registers a Runner for a specific RunnableType."""

    def decorator(cls: type[Runner]):
        if typ in _runners:
            raise ValueError(f"runner already registered for {typ}: {_runners[typ]!r}")
        _runners[typ] = cls
        return cls

    return decorator


class RuntimeRunner:
    """
    The runtime processes one top-level Run (or mini run for snippets) at a time.
    Inner runs may run in parallel if they are ready and read-only.

    NOTE :Robustness :Architecture: separate transactions for session and user edits?
    NOTE :Incomplete: respect RunOptions.max_concurrency
    NOTE :Architecture: Run started/terminated/... epochs are relative to session
        (meaning if we make local edits during a Run, the terminated_epoch is still the same)
    """

    def __init__(
        self, *, session: Session, oracle: Oracle, glbls: Mapping[str, Any] = CODE_GLOBALS
    ):
        self.session = session
        self.oracle = oracle
        self.glbls = glbls

        # ensure runners are imported
        if not _runners:
            import bench.runtime.code  # noqa: F401, RUF100
            import bench.runtime.step  # noqa: F401, RUF100
            import bench.runtime.text  # noqa: F401, RUF100

    async def _do_run_tracked(self, runner: Runner, handle: RunHandle):
        """
        Runs a coroutine as a tracked Run, retrying automatically.
        """
        run = handle.run
        assert run is not None, f"missing run in {handle!r}"

        # update context
        run.client_ptr = self.session.client_ptr
        run.machine_ptr = self.session.machine_ptr
        run.server_ptr = self.session.server_ptr
        run.user_ptr = self.session.user_ptr

        # start if not yet started
        if not run.started_at:
            run.started_epoch = self.session.epoch
            run.started_at = self.oracle.utc()
        run.status = run.current_status = RunStatus.RUNNING

        # actually attempt Run
        try:
            await self._do_run_untracked(runner, handle)
        finally:
            last_attempt = handle.current_attempt
            assert last_attempt is not None, f"no last attempt for run {handle!r}"
            run.attempts = handle.attempts
            run.outputs = handle.outputs
            run.error = handle.error
            run.status = run.current_status = handle.status
            run.duration = last_attempt.duration
            run.terminated_at = last_attempt.terminated_at
            run.terminated_epoch = last_attempt.terminated_epoch

    async def _do_run_untracked(self, runner: Runner, handle: RunHandle):
        """
        Runs a coroutine as an untracked run, retrying automatically.
        """
        handle.status = RunStatus.RUNNING
        retry = handle.options.to_retry().new(self.oracle, attempt=len(handle.attempts))
        # set active run
        if handle.run is not None:
            active_run_token = self.session._active_run.set(handle.run)
        else:
            active_run_token = None
        # run in attempt loop
        try:
            while retry.should_retry:
                with tracer.start_as_current_span(
                    "runner.attempt", attributes={"attempt": retry.attempt, "runner": repr(runner)}
                ):
                    retry.on_attempt()
                    attempt = RunAttempt(
                        status=RunStatus.RUNNING,
                        started_at=self.oracle.utc(),
                        started_epoch=self.session.epoch,
                    )
                    handle.attempts.append(attempt)
                    try:
                        await runner.run()
                        attempt.status = RunStatus.COMPLETED
                        break  # success
                    except Exception as e:
                        attempt.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                        attempt.status = RunStatus.FAILED
                        if not retry.on_error(e):
                            raise
                    finally:
                        attempt.terminated_at = self.oracle.utc()
                        attempt.terminated_epoch = self.session.epoch
                        assert attempt.started_at, f"missing started_at for attempt {attempt!r}"
                        attempt.duration = (
                            attempt.terminated_at - attempt.started_at
                        ).total_seconds()
            else:
                raise retry.to_error()
        finally:
            # run outcome = last attempt
            last_attempt = handle.current_attempt
            assert last_attempt is not None, f"missing last attempt for run {handle!r}"
            handle.status = last_attempt.status
            handle.error = last_attempt.error
            # reset active run
            if active_run_token is not None:
                self.session._active_run.reset(active_run_token)

    @tracer.start_as_current_span("runner.run")
    async def _do_run(self, handle: RunHandle):
        """Runs something runnable, considering its dependencies and run options."""

        # TODO :Incomplete: prepare run handle context (Runner.prepare?)

        runner_cls = _runners.get(handle.runnable.typ)
        if runner_cls is None:
            raise NotRunnableError(f"no runner for {handle!r}: {handle.runnable.typ}")
        trace.get_current_span().set_attribute("runner_cls", str(runner_cls))

        # run it
        async with self.session.unsuspended():
            runner = runner_cls(self, self.session, handle)
            trace.get_current_span().set_attribute("runner", repr(runner))
            if handle.run:
                await self._do_run_tracked(runner, handle)
            else:
                await self._do_run_untracked(runner, handle)
            # TODO :Performance!: support optimistic run-ahead and commit in background
            #  (rewind on failure or just fail the originating run?)
            await self.session.commit()

    #
    # High level Run helpers
    #

    async def run(
        self, run: Run | Block | Step, *, inputs: Any | None = None, suppress_error: bool = False
    ):
        """Auto-run whatever runnable node."""
        if not isinstance(run, Run):
            run = Run.from_runnable(run, inputs=inputs)
        await self.process_run(run, suppress_error=suppress_error)
        return run

    @tracer.start_as_current_span("runner.process_run")
    async def process_run(self, run: Run, *, suppress_error: bool):
        """Start or resumes a new top-level Run in this Runner. Returns on halt or termination."""
        # NOTE: Robustness: don't commit the entire session on failure?
        try:
            if run.kind == RunKind.BLOCK:
                block = run.block
                if block is None:
                    raise NotRunnableError(f"run {run!r} has no block")
                if block.type == BlockType.CODE:
                    has_function_fields = any(
                        f.zone in (FieldZone.INPUT, FieldZone.OUTPUT) for f in block.fields
                    )
                    code_kind = CodeKind.FUNCTION if has_function_fields else CodeKind.SCRIPT
                    runnable = RunnableState(
                        id=block.id,
                        typ=(RunnableKind.CODE, code_kind),
                        node=block,
                        code=block.code,
                        text=block.text,
                    )
                    handle = RunHandle(
                        id=run.id,
                        runnable=runnable,
                        options=run.options or DEFAULT_CODE_RUN_OPTIONS,
                        parent=None,
                        status=run.status,
                        inputs=run.inputs,
                        attempts=list(run.attempts),
                        run=run,
                    )
                    await self._do_run(handle)
                else:
                    raise NotRunnableError(f"block is not runnable: {block!r}")
            else:
                raise NotRunnableError(f"unexpected run: {run!r}")
            logger.info("runner.process_run", run=run, span="current")
        except RunHaltedError:
            # nothing to do?
            logger.info("runner.process_run.halt", run=run, span="current")
        except BenchError as e:
            # re-raised inner error
            async with self.session.unsuspended():
                if run.current_status != RunStatus.FAILED:
                    run.fail(RunError.from_exception(RunErrorKind.RUNTIME, e))
                await self.session.commit()
            logger.error("runner.process_run.error", run=run, exc_info=e, span="current")
            if not suppress_error:
                raise
        except Exception as e:
            # some internal error
            async with self.session.unsuspended():
                if run.current_status != RunStatus.FAILED:
                    run.fail(RunError.from_exception(RunErrorKind.INTERNAL, e))
                await self.session.commit()
            logger.error("runner.process_run.internal_error", run=run, exc_info=e, span="current")
            if not suppress_error:
                raise

    async def pause_run(self, run: Run):
        """Pause a Run currently executing in this Runner."""
        raise NotImplementedError

    async def abort_run(self, run: Run):
        """Abort a Run currently executing in this Runner."""
        raise NotImplementedError
