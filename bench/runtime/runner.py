import abc
import dataclasses
from contextvars import ContextVar
from dataclasses import dataclass
from typing import Any, Mapping
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.block import Block
from bench.language.code import Code, CodeType
from bench.language.const import BenchError, RunErrorKind, RunStatus
from bench.language.field import TypeInfoBase
from bench.language.log import LogInfo
from bench.language.run import ModelProvider, Run, RunAttempt, RunError, RunKind, RunOptions
from bench.language.session import Session
from bench.language.step import Step, StepType
from bench.language.text import Text
from bench.language.value import ValueObject
from bench.runtime.compiler import CompiledCode
from bench.runtime.core import (
    BASE_RUN_OPTIONS_BY_KIND,
    DYNAMIC_CODE_GLOBALS,
    STATIC_CODE_GLOBALS,
    RunImpossibleError,
)
from bench.utils.oracle import Oracle
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

RunnableNode = Block | Step
RunKey = CodeType | StepType | ModelProvider | None


@dataclass(slots=True)
class RunnableState[T: RunnableNode]:
    """The state of some runnable unit (Node or something within)."""

    id: UUID
    kind: RunKind
    key: RunKey | None
    node: T
    code: Code | None = None
    text: Text | None = None
    compiled: "CompiledCode | None" = None
    variables: ValueObject | None = None

    # outputs
    exports: Mapping[str, Any] | None = None  # for scripts
    last_expr_value: Any | None = None  # for snippets

    def __str__(self):
        type_str = (
            f"{self.kind.bench_name}:{self.key.bench_name}" if self.key else self.kind.bench_name
        )
        return f"{type_str}: {self.id} (in {self.node})"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}"


@dataclass(slots=True)
class RunHandle[T: RunnableNode]:
    """A specific run (tracked or untracked)."""

    id: UUID  # Run.id if tracked, new otherwise
    state: RunnableState[T]
    options: RunOptions
    parent: "RunHandle | None"  # if nested
    status: RunStatus
    inputs: ValueObject | None = None
    input_type: TypeInfoBase | None = None
    outputs: ValueObject | None = None
    output_type: TypeInfoBase | None = None
    error: RunError | None = None
    attempts: list[RunAttempt] = dataclasses.field(default_factory=list)
    logs: list[LogInfo] = dataclasses.field(default_factory=list)
    run: Run | None = None  # if tracked

    def __str__(self):
        str_parts: list[str] = [f"runnable={self.state!r}", f"options={self.options!r}"]
        if self.attempts:
            str_parts.append(f"attempts={self.attempts}")
        if self.run:
            str_parts.append(f"run={self.run}")
        return ", ".join(str_parts)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}"

    @property
    def kind(self) -> RunKind:
        return self.state.kind

    @property
    def key(self) -> RunKey | None:
        return self.state.key

    @property
    def current_attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None


class Runner[T: RunnableNode](abc.ABC):
    """A runner to process a single runnable unit once."""

    __slots__ = ("handle", "runtime", "session", "state")

    def __init__(self, runner: "RuntimeRunner", session: Session, handle: RunHandle[T]):
        self.runtime = runner
        self.session = session
        self.handle = handle
        self.state = handle.state

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
        return self.state.node

    @property
    def code(self) -> Code | None:
        return self.state.code

    @property
    def text(self) -> Text | None:
        return self.state.text

    @property
    def variables(self) -> ValueObject | None:
        return self.state.variables

    @property
    def status(self) -> RunStatus:
        return self.handle.status

    @property
    def input_type(self) -> TypeInfoBase | None:
        return self.handle.input_type

    @property
    def output_type(self) -> TypeInfoBase | None:
        return self.handle.output_type

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


_runners: dict[tuple[RunKind, RunKey | None], type[Runner]] = {}


def runner(kind: RunKind, typ: RunKey | None = None):
    """Registers a Runner for a specific RunnableType."""

    def decorator(cls: type[Runner]):
        if (kind, typ) in _runners:
            raise ValueError(f"runner already registered for {typ}: {_runners[(kind, typ)]!r}")
        _runners[(kind, typ)] = cls
        return cls

    return decorator


class RuntimeRunner:
    """
    The runtime processes one top-level Run (or mini run for snippets) at a time.
    Inner runs may run in parallel if they are ready and read-only.
    There may only be one RuntimeRunner per Session at a time.

    NOTE :Robustness :Architecture: separate transactions for session and user edits?
    NOTE :Incomplete: respect RunOptions.max_concurrency
    NOTE :Architecture: Run started/terminated/... epochs are relative to session
        (meaning if we make local edits during a Run, the terminated_epoch is still the same)
    """

    def __init__(
        self,
        *,
        session: Session,
        oracle: Oracle,
        static_glbls: Mapping[str, Any] = STATIC_CODE_GLOBALS,
        dynamic_glbls: Mapping[str, Any] = DYNAMIC_CODE_GLOBALS,
    ):
        assert session.package is not None, f"{session!r} is not attached"
        self.session = session
        self.package = session.package
        self.oracle = oracle
        self.static_glbls = static_glbls
        self.dynamic_glbls = dynamic_glbls
        self.combined_glbls = {**static_glbls, **dynamic_glbls}

        # session (maybe this should happen in some init?)
        assert session._runner is None, f"{session!r} already has a runner"
        self.session._runner = self
        self._active_run_handle: ContextVar[RunHandle | None] = ContextVar("active_run_handle")
        self._active_runs_by_id: dict[UUID, RunHandle] = {}

        # ensure runners are imported
        if not _runners:
            import bench.runtime.code  # noqa: F401, RUF100
            import bench.runtime.step  # noqa: F401, RUF100
            import bench.runtime.text  # noqa: F401, RUF100

    def __str__(self):
        return f"{len(self._active_runs_by_id)} active, {self.session!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def active_run_handle(self) -> RunHandle | None:
        return self._active_run_handle.get(None)

    @property
    def active_run(self) -> Run | None:
        handle = self._active_run_handle.get(None)
        return handle.run if handle else None

    @tracer.start_as_current_span("runner.make")
    async def make_run_handle(
        self,
        # state
        kind: RunKind,
        *,
        node: Block | Step,
        track: bool,
        key: RunKey | None = None,
        code: Code | None = None,
        text: Text | None = None,
        variables: ValueObject | None = None,
        # handle
        inputs: ValueObject | None = None,
        options: RunOptions | None = None,
        run: Run | None = None,
    ) -> RunHandle:
        """Get/create/recover the state for some runnable."""
        # NOTE :Incomplete: re-use existing state for block code scripts (to get exports)

        # make state
        if kind == RunKind.CODE:
            code = code or node.code
            if (isinstance(node, Block) and node.has_function_fields) or isinstance(node, Step):
                key = CodeType.FUNCTION
            else:
                key = CodeType.SCRIPT
        elif kind == RunKind.TEXT:
            text = text or node.text
        state = RunnableState(
            id=node.id,
            kind=kind,
            key=key,
            node=node,
            code=code,
            text=text,
            variables=variables,
        )

        # make handle
        handle = RunHandle(
            id=run.id if run else UUIDT(),
            state=state,
            options=BASE_RUN_OPTIONS_BY_KIND[kind].override(options),
            parent=self.active_run_handle,
            status=run.status if run else RunStatus.SCHEDULED,
            input_type=node.input_type,
            output_type=node.output_type,
            inputs=inputs,
            attempts=run.attempts if run else [],
            run=run,
        )
        if track and run is None:
            pass  # NOTE :Incomplete: nested (tracked) run"

        return handle

    async def make_run_handle_from_run(self, run: Run):
        node = run.step or run.block
        if node is None:
            raise RunImpossibleError(f"no node for {run!r}")  # default to package?
        options = node.run_options.override(run.options) if node.run_options else run.options
        return await self.make_run_handle(
            run.kind,
            node=node,
            code=run.code,
            run=run,
            inputs=run.inputs,
            options=options,
            track=True,
        )

    async def _do_run_retrying(self, runner: Runner):
        """
        Runs a handle in a Runner, retrying automatically and updating the RunHandle along the way.
        """
        handle = runner.handle
        handle.status = RunStatus.RUNNING
        retry = handle.options.to_retry().new(self.oracle, attempt=len(handle.attempts))
        # set active run
        active_run_handle_token = self._active_run_handle.set(handle)
        # run in attempt loop
        log = logger.bind(runner=runner, handle=handle, retry=retry)
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
                        log.debug("runner.attempt", attempt=attempt, span="current")
                        break  # success
                    except Exception as e:
                        error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                        attempt.error = error
                        attempt.status = RunStatus.FAILED
                        log.debug(
                            "runner.attempt.failed", attempt=attempt, exc_info=e, span="current"
                        )
                        if not error.is_retryable or (
                            not retry.on_error(e)
                            and not (error.type and error.type in handle.options.retry_on)
                        ):
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
            self._active_run_handle.reset(active_run_handle_token)

    async def _do_run_retrying_tracked(self, runner: Runner):
        """
        Runs a handle in A Runner, retrying automatically and updating the Run along the way.
        """
        handle = runner.handle
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
            await self._do_run_retrying(runner)
        finally:
            last_attempt = handle.current_attempt
            assert last_attempt is not None, f"no last attempt for run {handle!r}"
            run.attempts = handle.attempts
            run.logs = handle.logs
            run.outputs = handle.outputs
            run.error = handle.error
            run.status = run.current_status = handle.status
            run.duration = last_attempt.duration
            run.terminated_at = last_attempt.terminated_at
            run.terminated_epoch = last_attempt.terminated_epoch

    @tracer.start_as_current_span("runner.run_handle")
    async def run_handle(self, handle: RunHandle):
        """Runs something runnable, considering its dependencies and run options."""

        # TODO :Incomplete: prepare run handle context (Runner.prepare?)

        runner_cls = _runners.get((handle.kind, handle.key))
        if runner_cls is None:
            raise RunImpossibleError(f"no runner for {handle!r}: {handle.state.key}")
        trace.get_current_span().set_attribute("runner_cls", str(runner_cls))

        # run it
        async with self.session.active():
            runner = runner_cls(self, self.session, handle)
            trace.get_current_span().set_attribute("runner", repr(runner))
            if handle.run is not None:
                await self._do_run_retrying_tracked(runner)
            else:
                await self._do_run_retrying(runner)
            # TODO :Performance!: support optimistic commit (commit in background, fail if failed)
            await self.session.commit()

    @tracer.start_as_current_span("runner.process_run")
    async def process_run(self, run: Run, *, suppress_error: bool) -> RunHandle | None:
        """Start or resume a top-level Run in this Runner. Returns on halt or termination."""
        handle = None
        async with self.session.active():
            try:
                handle = await self.make_run_handle_from_run(run=run)
                await self.run_handle(handle)
                logger.info("runner.process_run", run=run, handle=handle, span="current")
            except (BenchError, ValueError, TypeError) as e:
                # NOTE: Robustness: should we really commit the entire session on failure?
                #  (maybe have some sort of atomic flag or context manager to prevent it as needed?)
                # re-raised inner user error
                if run.current_status != RunStatus.FAILED:
                    run.fail(RunError.from_exception(RunErrorKind.RUNTIME, e))
                await self.session.commit()
                logger.info("runner.process_run.error", run=run, exc_info=e, span="current")
                if not suppress_error:
                    raise
            except Exception as e:
                # some unexpected internal error
                if run.current_status != RunStatus.FAILED:
                    run.fail(RunError.from_exception(RunErrorKind.INTERNAL, e), _force=True)
                await self.session.commit()
                logger.error(
                    "runner.process_run.internal_error", run=run, exc_info=e, span="current"
                )
                if not suppress_error:
                    raise
            return handle

    async def pause_run(self, run: Run):
        """Pause a Run currently executing in this Runner."""
        raise NotImplementedError

    async def abort_run(self, run: Run):
        """Abort a Run currently executing in this Runner."""
        raise NotImplementedError

    async def run(
        self, run: Run | Block | Step, *, inputs: Any | None = None, return_error: bool = False
    ) -> RunHandle:
        """Auto-run wrapper for some runnable node (may already be in another run)."""
        if not isinstance(run, Run):
            run = Run.from_runnable(run, inputs=inputs, parent=self.active_run)
        handle = await self.process_run(run, suppress_error=return_error)
        assert handle is not None, f"no handle for {run!r}"
        return handle
