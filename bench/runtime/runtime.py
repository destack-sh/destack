import asyncio
from contextvars import ContextVar
from datetime import datetime
from typing import Any, Mapping
from uuid import UUID

import structlog
from opentelemetry import baggage, context, trace

from bench.language.const import BenchError, ObjectKind, RunErrorKind, RunStatus
from bench.language.interrupt import RUN_STATUS_BY_INTERRUPT_KIND
from bench.language.run import Run, RunAttempt, RunError, RunKind, RunnableNode, RunOptions
from bench.language.session import Session
from bench.language.validation import ValidationError, on_invalid_raise
from bench.language.value import CustomObject, check_value
from bench.runtime.cache import Cache
from bench.runtime.core import (
    ATTEMPT_ONCE,
    ATTEMPT_THRICE,
    DYNAMIC_CODE_GLOBALS,
    STATIC_CODE_GLOBALS,
    NonRetryableError,
    RetryableError,
)
from bench.runtime.runner import Interrupted, Runner, RunnerHook, run_from_node, runner_from_run
from bench.utils.oracle import Oracle

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
        cache: Cache,
        oracle: Oracle,
        static_glbls: Mapping[str, Any] = STATIC_CODE_GLOBALS,
        dynamic_glbls: Mapping[str, Any] = DYNAMIC_CODE_GLOBALS,
    ):
        assert session.package is not None, f"{session!r} is not attached"
        self.session = session
        self.cache = cache
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
        return runner.tracked_run if runner else None

    @tracer.start_as_current_span("runtime.run_runner.retrying")
    async def _do_run(self, runner: Runner):
        """Runs a a Runner, retrying automatically and updating the Runner along the way."""
        # NOTE :Performance: track attempt as efficiently as possible :RuntimeHotPath

        # check inputs
        if runner.input_type is not None:
            with tracer.start_as_current_span("runtime.check_inputs"):
                inputs = runner.inputs or CustomObject.new(ObjectKind.INPUT, {}, runner.input_type)
                try:
                    check_value(inputs, runner.input_type, on_invalid_raise)
                except ValidationError as e:
                    runner.status = RunStatus.FAILED
                    runner.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                    return

        # set active run
        active_run_runner_token = self._active_runner.set(runner)

        # run attempts
        runner.status = RunStatus.RUNNING
        retry = runner.options.to_retry().new(self.oracle, attempt=len(runner.attempts))
        last_attempt = runner.current_attempt
        if last_attempt is not None and last_attempt.status.is_interrupted:
            # resume interrupted attempt
            attempt = last_attempt
            retry.attempt -= 1  # don't count interrupted attempt
        else:
            # start new attempt
            attempt = RunAttempt(
                status=RunStatus.RUNNING,
                started_epoch=self.session.epoch,
                _skip_validate_self=True,
            )
            runner.attempts.append(attempt)
        log = logger.bind(runner=runner, retry=retry)
        try:
            while retry.should_retry:
                with tracer.start_as_current_span(
                    "runtime.attempt", attributes={"attempt": retry.attempt, "runner": repr(runner)}
                ):
                    retry.on_attempt()
                    started_at: datetime | None = None
                    terminated_at: datetime | None = None
                    try:
                        if runner.is_cancelled:
                            raise asyncio.CancelledError()
                        with tracer.start_as_current_span("runtime.attempt.run"):
                            started_at = self.oracle.utc()
                            attempt._do_set("started_at", started_at, validate=False)
                            runner.task = asyncio.create_task(runner.run())
                            await runner.task
                            terminated_at = self.oracle.utc()
                        if runner.output_type is not None:
                            with tracer.start_as_current_span("runtime.check_outputs"):
                                if runner.outputs is None:
                                    runner.outputs = CustomObject.new(
                                        ObjectKind.OUTPUT, {}, runner.output_type
                                    )
                                check_value(runner.outputs, runner.output_type, on_invalid_raise)
                        attempt._do_set("status", RunStatus.COMPLETED, validate=False)
                        log.debug("runtime.attempt", attempt=attempt, span="current")
                        return  # success
                    except asyncio.CancelledError as e:
                        error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                        attempt._do_set("status", RunStatus.ABORTED, validate=False)
                        attempt._do_set("error", error, validate=False)
                        log.debug(
                            "runtime.attempt.aborted", attempt=attempt, exc_info=e, span="current"
                        )
                        if runner.is_nested:
                            raise  # bubble up if not root runner
                        else:
                            return  # give up directly
                    except Interrupted as e:
                        status = RUN_STATUS_BY_INTERRUPT_KIND[e.interrupt.kind]
                        attempt._do_set("status", status, validate=False)
                        attempt._do_set("interrupted_at", self.oracle.utc(), validate=False)
                        attempt._do_set("interrupt", e.interrupt, validate=False)
                        if runner.is_nested:
                            raise  # bubble up if not root runner
                        else:
                            return  # stop directly
                    except BaseException as e:
                        error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                        attempt._do_set("error", error, validate=False)
                        attempt._do_set("status", RunStatus.FAILED, validate=False)
                        log.debug(
                            "runtime.attempt.failed", attempt=attempt, exc_info=e, span="current"
                        )
                        if not error.is_retryable or (
                            not retry.on_error(e)
                            and not (error.type and error.type in runner.options.retry_on)
                        ):
                            raise  # give up if not retryable (anymore)
                    finally:
                        runner.task = None
                        if runner.status.is_terminal:
                            if terminated_at is None:
                                terminated_at = self.oracle.utc()
                            attempt._do_set("terminated_at", terminated_at, validate=False)
                            attempt._do_set("terminated_epoch", self.session.epoch, validate=False)
                            if started_at is not None:
                                attempt._do_set(
                                    "duration", terminated_at - started_at, validate=False
                                )
                # begin new attempt
                attempt = RunAttempt(
                    status=RunStatus.RUNNING,
                    started_epoch=self.session.epoch,
                    _skip_validate_self=True,
                )
                runner.attempts.append(attempt)
            else:
                last_attempt = runner.current_attempt
                if last_attempt and last_attempt.error:
                    raise RetryableError(message=last_attempt.error.title, error=last_attempt.error)
                else:
                    # shouldn't actually get here if RetryOptions.max_attempts > 0
                    raise NonRetryableError(message="retry exhausted")
        finally:
            # run outcome = last attempt
            last_attempt = runner.current_attempt
            assert last_attempt is not None, f"missing last attempt for run {runner!r}"
            runner.status = last_attempt.status
            runner.error = last_attempt.error
            # reset active run
            self._active_runner.reset(active_run_runner_token)

    @tracer.start_as_current_span("runtime.run_runner.tracked")
    async def _do_run_tracked(self, runner: Runner):
        """Runs a Runner, retrying automatically and updating the Run along the way."""
        # NOTE :Performance: update the Run as efficiently as possible :RuntimeHotPath
        # NOTE :UX :Performance: commit optimistically ideally only while inside user code
        #  (while we're inside a leaf Runner.run, but not while updating/creating Runs,
        #   so for instance inside a Flow we should wait for all initial Steps to start somehow)

        run = runner.tracked_run
        assert run is not None, f"missing run in {runner!r}"
        context.attach(baggage.set_baggage("run_id", str(run.id)))

        # update context
        if run.session_id != self.session.id:
            run._do_set("session", self.session, validate=False)
        run._do_set("client_ptr", self.session.client_ptr, validate=False)
        run._do_set("machine_ptr", self.session.machine_ptr, validate=False)
        run._do_set("server_ptr", self.session.server_ptr, validate=False)
        run._do_set("user_ptr", self.session.user_ptr, validate=False)

        # start if not yet started
        if run.started_at is None:
            run._do_set("started_epoch", self.session.epoch, validate=False)
            run._do_set("started_at", self.oracle.utc(), validate=False)
        run._do_set("status", RunStatus.RUNNING, validate=False)

        # commit intermediate session edits
        self.session.commit_optimistic()

        # actually attempt Run
        try:
            await self._do_run(runner)
        finally:
            # get status from last runner / last attempt
            run._do_set("attempts", runner.attempts, validate=False)
            run._do_set("logs", runner.logs, validate=False)
            run._do_set("inputs", runner.inputs, validate=False)
            run._do_set("outputs", runner.outputs, validate=False)
            run._do_set("error", runner.error, validate=False)
            run._do_set("status", runner.status, validate=False)
            last_attempt = runner.current_attempt
            if last_attempt is not None:
                if runner.status.is_interrupted:
                    run._do_set("interrupted_at", last_attempt.interrupted_at, validate=False)
                    run._do_set("interrupt", last_attempt.interrupt, validate=False)
                elif runner.status.is_terminal:
                    run._do_set("terminated_at", last_attempt.terminated_at, validate=False)
                    run._do_set("terminated_epoch", last_attempt.terminated_epoch, validate=False)
                    if last_attempt.duration is not None:
                        run._do_set("duration", last_attempt.duration, validate=False)
            elif run.terminated_at is not None:
                # didn't make an attempt, but we have a terminated_at
                run._do_set("duration", run.terminated_at - run.started_at)  # type: ignore
            elif run.status.is_terminal:
                # didn't make an attempt
                run._do_set("terminated_at", self.oracle.utc(), validate=False)
                run._do_set("terminated_epoch", self.session.epoch, validate=False)
                run._do_set("duration", run.terminated_at - run.started_at)  # type: ignore

            # commit intermediate session edits
            self.session.commit_optimistic()

    @tracer.start_as_current_span("runtime.run_runner")
    async def run_runner(self, runner: Runner, hook: RunnerHook | None = None):
        """Runs something runnable, considering its dependencies and run options."""
        async with self.session.active():
            self._active_runners_by_id[runner.id] = runner
            exc = None
            try:
                trace.get_current_span().set_attribute("runner", repr(runner))
                if runner.tracked_run is not None:
                    await self._do_run_tracked(runner)
                else:
                    await self._do_run(runner)
            except Exception as e:
                exc = e
                raise
            finally:
                self._active_runners_by_id.pop(runner.id, None)
                if not runner.is_tracked and runner.parent is not None:
                    # add inner spans/logs/events to parent
                    runner.parent.logs.extend(runner.logs)
                    runner.parent.events.extend(runner.events)
                    runner.parent.spans.extend(runner.spans)
                if hook is not None:
                    hook(runner, exc)

    def create_runner(self, runner: Runner, on_stop: RunnerHook | None = None) -> Runner:
        """Create a new Runner and run it."""
        runner.outer_task = asyncio.create_task(self.run_runner(runner, hook=on_stop))
        return runner

    @tracer.start_as_current_span("runtime.run")
    async def run(
        self,
        run: Run | RunnableNode,
        *,
        options: RunOptions | None = None,
        inputs: Any | None = None,
        return_error: bool = False,
        optimistic: bool = False,
    ) -> Runner | None:
        """Start or resume a top-level Run in this Runtime."""
        if not isinstance(run, Run):
            if options is None:
                options = ATTEMPT_THRICE if run.run_kind == RunKind.ACTION else ATTEMPT_ONCE
            run = run_from_node(run, options=options, inputs=inputs, parent=self.active_run)
        runner = None
        async with self.session.active():
            try:
                runner = runner_from_run(runtime=self, run=run, track=True)
                await self.run_runner(runner)
                logger.info("runtime.run", run=run, runner=runner, span="current")
            except (BenchError, ValueError, TypeError) as e:
                # re-raised inner user error
                if run.status != RunStatus.FAILED:
                    run.status = RunStatus.FAILED
                    run.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                self.session.commit_optimistic()
                logger.info("runtime.run.error", run=run, exc_info=e, span="current")
                if not return_error:
                    raise
            except BaseException as e:
                # some unexpected internal error
                if run.status != RunStatus.FAILED:
                    run.status = RunStatus.FAILED
                    run.error = RunError.from_exception(RunErrorKind.INTERNAL, e)
                self.session.commit_optimistic()
                logger.error("runtime.run.internal_error", run=run, exc_info=e, span="current")
                if not return_error:
                    raise
            finally:
                if not optimistic:
                    await self.session.commit()
        return runner

    async def pause(self, run: Run):
        """Pause a Run currently executing in this Runtime."""
        raise NotImplementedError

    async def abort(self, run: Run):
        """Abort a Run currently executing in this Runtime (and any inside it)."""
        root_runner = self._active_runners_by_id.get(run.id)
        if root_runner is None:
            raise RuntimeError(f"no active runner for {run!r} in {self!r}")
        for runner in reversed(list(root_runner.walk())):
            if not runner.status.is_terminal:
                runner.cancel()
                logger.debug("runtime.run.abort", runner=runner)
