import asyncio
from contextvars import ContextVar
from typing import Any, Mapping
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.block import Block
from bench.language.code import Code, CodeType
from bench.language.const import BenchError, RunErrorKind, RunStatus
from bench.language.flow import Step
from bench.language.run import Run, RunAttempt, RunError, RunKind, RunOptions
from bench.language.session import Session
from bench.language.text import Text
from bench.language.validation import ValidationError, on_invalid_raise
from bench.language.value import ValueObject, check_value
from bench.runtime.core import (
    BASE_RUN_OPTIONS_BY_KIND,
    DYNAMIC_CODE_GLOBALS,
    STATIC_CODE_GLOBALS,
    RunImpossibleError,
)
from bench.runtime.runner import Runner, RunSubtype, _runners
from bench.utils.oracle import Oracle
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Runtime:
    """
    The runtime for executing Runs.
    One top-level Run/Runner is processed at a time.
    Inner runs may run in parallel in some cases.
    A Runtime is exclusively associated with one Session.

    NOTE :Robustness :Architecture: separate transactions for session and other edits?
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
        assert session._runtime is None, f"{session!r} already in runtime {session._runtime!r}"
        self.session._runtime = self
        self._active_runner: ContextVar[Runner | None] = ContextVar("active_runner")
        self._active_runners_by_id: dict[UUID, Runner] = {}

        # ensure runners are imported
        if not _runners:
            import bench.runtime.code  # noqa: F401, RUF100
            import bench.runtime.flow  # noqa: F401, RUF100
            import bench.runtime.text  # noqa: F401, RUF100

    def __str__(self):
        return f"{len(self._active_runners_by_id)} active, {self.session!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def active_runner(self) -> Runner | None:
        return self._active_runner.get(None)

    @property
    def active_run(self) -> Run | None:
        runner = self._active_runner.get(None)
        return runner.run if runner else None

    @tracer.start_as_current_span("runner.make")
    async def make_runner(
        self,
        # state
        kind: RunKind,
        *,
        node: Block | Step,
        track: bool,
        subtype: RunSubtype | None = None,
        code: Code | None = None,
        text: Text | None = None,
        variables: ValueObject | None = None,
        # runner
        inputs: ValueObject | None = None,
        options: RunOptions | None = None,
        run: Run | None = None,
    ) -> Runner:
        """Make the Runner to run some runnable in a Run."""

        # figure out which runner we need
        if kind == RunKind.CODE:
            code = code or node.code or Code.empty()
            if (isinstance(node, Block) and node.has_function_fields) or isinstance(node, Step):
                subtype = CodeType.FUNCTION
            else:
                subtype = CodeType.SCRIPT
        elif kind == RunKind.TEXT:
            text = text or node.text
            if options and options.model_options:
                subtype = options.model_options.provider
        elif kind == RunKind.STEP:
            assert isinstance(node, Step), f"unexpected node type: {node!r}"
            subtype = node.type

        # make runner with state
        runner_cls = _runners.get((kind, subtype))
        if runner_cls is None:
            raise RunImpossibleError(f"no runner for {kind.bench_name}:{subtype}")
        # NOTE :Incomplete: re-use existing state sometimes (e.g., code script exports)
        cache = runner_cls.cache_cls(
            id=node.id,
            kind=kind,
            subtype=subtype,
            node=node,
            code=code,
            text=text,
            variables=variables,
        )
        runner = runner_cls(
            id=run.id if run else UUIDT(),
            runtime=self,
            cache=cache,
            options=BASE_RUN_OPTIONS_BY_KIND[kind].override(options),
            parent=self.active_runner,
            status=run.status if run else RunStatus.SCHEDULED,
            input_type=node.input_type,
            output_type=node.output_type,
            inputs=inputs,
            attempts=run.attempts if run else [],
            run=run,
        )

        # nest active Runners/Runs
        active_runner = self.active_runner
        if active_runner is not None:
            active_runner.runs.append(runner)
        if track and run is None:
            if active_runner is not None:
                assert active_runner.run is not None, f"{active_runner!r} has no Run"
                parent_run = active_runner.run
            else:
                parent_run = None
            run = Run(
                parent=parent_run or self.session.package,
                kind=kind,
                block=node if isinstance(node, Block) else None,
                step=node if isinstance(node, Step) else None,
                options=runner.options,
                status=runner.status,
                inputs=runner.inputs,
                attempts=runner.attempts,
            )
            self.session._create(run)
            runner.run = run

        return runner

    async def make_runner_from_run(self, run: Run):
        """Make a Runner from a Run."""
        node = run.step or run.block
        if node is None:
            raise RunImpossibleError(f"no node for {run!r}")  # default to package?
        options = node.run_options.override(run.options) if node.run_options else run.options
        if run.inputs is None and run.input_type is not None:
            inputs = ValueObject.new({}, run.input_type)
        else:
            inputs = run.inputs
        return await self.make_runner(
            run.kind,
            node=node,
            code=run.code,
            run=run,
            inputs=inputs,
            options=options,
            track=True,
        )

    async def _do_run_once_retrying(self, runner: Runner):
        """Runs a a Runner, retrying automatically and updating the Runner along the way."""
        # check inputs
        if runner.input_type is not None:
            inputs = runner.inputs or ValueObject.new({}, runner.input_type)
            try:
                check_value(inputs, runner.input_type, on_invalid_raise)
            except ValidationError as e:
                runner.status = RunStatus.FAILED
                runner.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                return

        # set active run
        active_run_runner_token = self._active_runner.set(runner)
        runner.status = RunStatus.RUNNING
        retry = runner.options.to_retry().new(self.oracle, attempt=len(runner.attempts))

        # run attempts
        log = logger.bind(runner=runner, retry=retry)
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
                    runner.attempts.append(attempt)
                    try:
                        runner.inner_task = asyncio.create_task(runner.run_once())
                        await runner.inner_task
                        if runner.output_type is not None:
                            check_value(runner.outputs, runner.output_type, on_invalid_raise)
                        attempt.status = RunStatus.COMPLETED
                        log.debug("runner.attempt", attempt=attempt, span="current")
                        break  # success
                    except asyncio.CancelledError as e:
                        error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                        attempt.status = RunStatus.ABORTED
                        attempt.error = error
                        log.debug(
                            "runner.attempt.aborted", attempt=attempt, exc_info=e, span="current"
                        )
                        raise  # give up (always)
                    except BaseException as e:
                        error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                        attempt.error = error
                        attempt.status = RunStatus.FAILED
                        log.debug(
                            "runner.attempt.failed", attempt=attempt, exc_info=e, span="current"
                        )
                        if not error.is_retryable or (
                            not retry.on_error(e)
                            and not (error.type and error.type in runner.options.retry_on)
                        ):
                            raise  # give up if not retryable (anymore)
                    finally:
                        runner.inner_task = None
                        attempt.terminated_at = self.oracle.utc()
                        attempt.terminated_epoch = self.session.epoch
                        assert attempt.started_at, f"missing started_at for attempt {attempt!r}"
                        attempt.duration = (
                            attempt.terminated_at - attempt.started_at
                        ).total_seconds()
            else:
                raise retry.to_error()  # give up
        finally:
            # run outcome = last attempt
            last_attempt = runner.current_attempt
            assert last_attempt is not None, f"missing last attempt for run {runner!r}"
            runner.status = last_attempt.status
            runner.error = last_attempt.error
            # reset active run
            self._active_runner.reset(active_run_runner_token)

    async def _do_run_once_retrying_tracked(self, runner: Runner):
        """Runs a Runner, retrying automatically and updating the Run along the way."""
        run = runner.run
        assert run is not None, f"missing run in {runner!r}"

        # update context
        run.client_ptr = self.session.client_ptr
        run.machine_ptr = self.session.machine_ptr
        run.server_ptr = self.session.server_ptr
        run.user_ptr = self.session.user_ptr

        # start if not yet started
        if not run.started_at:
            run.started_epoch = self.session.epoch
            run.started_at = self.oracle.utc()
        run.status = RunStatus.RUNNING

        # actually attempt Run
        try:
            await self._do_run_once_retrying(runner)
        finally:
            run.attempts = runner.attempts
            run.logs = runner.logs
            run.inputs = runner.inputs
            run.outputs = runner.outputs
            run.error = runner.error
            run.status = runner.status
            last_attempt = runner.current_attempt
            if last_attempt is not None:
                # may not have a last attempt if we didn't even try
                run.terminated_at = last_attempt.terminated_at
                run.terminated_epoch = last_attempt.terminated_epoch
            if run.terminated_at is not None:
                run.duration = (run.terminated_at - run.started_at).total_seconds()

    @tracer.start_as_current_span("runner.run_runner")
    async def run_runner(self, runner: Runner):
        """Runs something runnable, considering its dependencies and run options."""

        # TODO :Incomplete: prepare run runner context (Runner.prepare?)
        #  (like for code we need its referenced imports/exports ready)

        # run it
        async with self.session.active():
            trace.get_current_span().set_attribute("runner", repr(runner))
            if runner.run is not None:
                await self._do_run_once_retrying_tracked(runner)
            else:
                await self._do_run_once_retrying(runner)
            # nocheckin :Performance! :Robustness: support optimistic & more robust commit
            #  (increment local epoch, handle concurrent commits/edits, ... :BetterCommit)
            await asyncio.shield(self.session.commit())

    @tracer.start_as_current_span("runner.process_run")
    async def run_run(self, run: Run, *, suppress_error: bool) -> Runner | None:
        """Start or resume a top-level Run in this Runner. Returns on halt or termination."""
        runner = None
        async with self.session.active():
            try:
                runner = await self.make_runner_from_run(run=run)
                await self.run_runner(runner)
                logger.info("runner.process_run", run=run, runner=runner, span="current")
            except (BenchError, ValueError, TypeError) as e:
                # NOTE: Robustness: should we really commit the entire session on failure?
                #  (maybe have some sort of atomic flag or context manager to prevent it as needed?)
                # re-raised inner user error
                if run.status != RunStatus.FAILED:
                    run.status = RunStatus.FAILED
                    run.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                await asyncio.shield(self.session.commit())
                logger.info("runner.process_run.error", run=run, exc_info=e, span="current")
                if not suppress_error:
                    raise
            except Exception as e:
                # some unexpected internal error
                if run.status != RunStatus.FAILED:
                    run.status = RunStatus.FAILED
                    run.error = RunError.from_exception(RunErrorKind.INTERNAL, e)
                await asyncio.shield(self.session.commit())
                logger.error(
                    "runner.process_run.internal_error", run=run, exc_info=e, span="current"
                )
                if not suppress_error:
                    raise
            return runner

    async def pause_run(self, run: Run):
        """Pause a Run currently executing in this Runner."""
        raise NotImplementedError

    async def abort_run(self, run: Run):
        """Abort a Run currently executing in this Runner."""
        raise NotImplementedError

    async def run(
        self, run: Run | Block | Step, *, inputs: Any | None = None, return_error: bool = False
    ) -> Runner:
        """Auto-run wrapper for some runnable node (may already be in another run)."""
        if not isinstance(run, Run):
            run = Run.from_runnable(run, inputs=inputs, parent=self.active_run)
        runner = await self.run_run(run, suppress_error=return_error)
        assert runner is not None, f"no runner for {run!r}"
        return runner
