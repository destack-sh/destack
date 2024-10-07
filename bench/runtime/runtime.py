import asyncio
from contextvars import ContextVar
from datetime import datetime
from typing import Any, Mapping, assert_never
from uuid import UUID

import structlog
from opentelemetry import baggage, context, trace

from bench.language.block import Block
from bench.language.code import Code
from bench.language.const import BenchError, RunErrorKind, RunStatus
from bench.language.flow import Step, StepType
from bench.language.run import ModelProvider, Run, RunAttempt, RunError, RunKind, RunOptions
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
from bench.runtime.runner import RunnableNode, Runner
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

    @tracer.start_as_current_span("runtime.make")
    async def make_runner(
        self,
        # state
        kind: RunKind,
        *,
        node: Block | Step,
        track: bool,
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
        elif kind == RunKind.TEXT:
            text = text or node.text
        elif kind == RunKind.STEP:
            assert isinstance(node, Step), f"unexpected node type: {node!r}"

        # make runner with state
        runner_cls = get_runner_cls(kind=kind, node=node, code=code, text=text, options=options)
        if runner_cls is None:
            raise RunImpossibleError(f"no runner for {kind.bench_name}:{node!r}")
        # NOTE :Incomplete: re-use existing state sometimes (e.g., code script exports)
        cache = runner_cls.cache_cls(
            id=node.id,
            kind=kind,
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
            attempts=list(run.attempts) if run else [],
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
            with tracer.start_as_current_span("runtime.create_run"):
                run = Run(
                    parent=parent_run or self.session.package,
                    kind=kind,
                    block=node if isinstance(node, Block) else None,
                    step=node if isinstance(node, Step) else None,
                    options=runner.options,
                    status=runner.status,
                    inputs=runner.inputs,
                    attempts=runner.attempts,
                    session=self.session,
                    _skip_validate_self=True,
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

    @tracer.start_as_current_span("runtime.run_runner.once_retrying")
    async def _do_run_once_retrying(self, runner: Runner):
        """Runs a a Runner, retrying automatically and updating the Runner along the way."""
        # check inputs
        if runner.input_type is not None:
            with tracer.start_as_current_span("runtime.check_inputs"):
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

        # NOTE :Performance: track attempt as efficiently as possible :RuntimeHotPath

        # run attempts
        log = logger.bind(runner=runner, retry=retry)
        try:
            while retry.should_retry:
                with tracer.start_as_current_span(
                    "runtime.attempt", attributes={"attempt": retry.attempt, "runner": repr(runner)}
                ):
                    retry.on_attempt()
                    started_at: datetime | None = None
                    terminated_at: datetime | None = None
                    attempt = RunAttempt(
                        status=RunStatus.RUNNING,
                        started_epoch=self.session.epoch,
                        _skip_validate_self=True,
                    )
                    runner.attempts.append(attempt)
                    try:
                        if runner.is_cancelled:
                            raise asyncio.CancelledError()
                        with tracer.start_as_current_span("runtime.attempt.run"):
                            started_at = self.oracle.utc()
                            attempt._do_set("started_at", started_at, validate=False)
                            runner.task = asyncio.create_task(runner.run_once())
                            await runner.task
                            terminated_at = self.oracle.utc()
                        if runner.output_type is not None:
                            with tracer.start_as_current_span("runtime.check_outputs"):
                                if runner.outputs is None:
                                    runner.outputs = ValueObject.new({}, runner.output_type)
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
                        if terminated_at is None:
                            terminated_at = self.oracle.utc()
                        attempt._do_set("terminated_at", terminated_at, validate=False)
                        attempt._do_set("terminated_epoch", self.session.epoch, validate=False)
                        if started_at is not None:
                            attempt._do_set("duration", terminated_at - started_at, validate=False)
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

    @tracer.start_as_current_span("runtime.run_runner.once_tracked")
    async def _do_run_once_retrying_tracked(self, runner: Runner):
        """Runs a Runner, retrying automatically and updating the Run along the way."""
        run = runner.run
        assert run is not None, f"missing run in {runner!r}"
        context.attach(baggage.set_baggage("run_id", str(run.id)))

        # NOTE :Performance: update the run as efficiently as possible :RuntimeHotPath
        # NOTE :UX :Performance: commit optimistically ideally only while inside user code
        #  (while we're inside a leaf Runner.run, but not while updating/creating Runs,
        #   so for instance inside a Flow we should wait for all initial Steps to start somehow)

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
            await self._do_run_once_retrying(runner)
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
    async def run_runner(self, runner: Runner):
        """Runs something runnable, considering its dependencies and run options."""

        # NOTE :Incomplete: prepare run runner context (Runner.prepare?)
        #  (for code, we need the referenced imports/exports ready)

        # run it
        async with self.session.active():
            self._active_runners_by_id[runner.id] = runner
            try:
                trace.get_current_span().set_attribute("runner", repr(runner))
                if runner.run is not None:
                    await self._do_run_once_retrying_tracked(runner)
                else:
                    await self._do_run_once_retrying(runner)
            finally:
                self._active_runners_by_id.pop(runner.id, None)

    @tracer.start_as_current_span("runtime.run")
    async def run(
        self,
        run: Run | Block | Step,
        *,
        inputs: Any | None = None,
        return_error: bool = False,
        optimistic: bool = False,
    ) -> Runner | None:
        """Start or resume a top-level Run in this Runtime. Returns on halt or termination."""
        if not isinstance(run, Run):
            run = Run.new(run, inputs=inputs, parent=self.active_run)
        runner = None
        async with self.session.active():
            try:
                runner = await self.make_runner_from_run(run=run)
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

    async def pause_run(self, run: Run):
        """Pause a Run currently executing in this Runtime."""
        raise NotImplementedError

    async def abort_run(self, run: Run):
        """Abort a Run currently executing in this Runtime (and any inside it)."""
        root_runner = self._active_runners_by_id.get(run.id)
        if root_runner is None:
            raise RuntimeError(f"no active runner for {run!r} in {self!r}")
        for runner in reversed(list(root_runner.walk())):
            runner.is_cancelled = True
            if runner.task is not None:
                runner.task.cancel()
            logger.debug("runtime.run.abort", runner=runner)


def get_runner_cls(
    *,
    kind: RunKind,
    node: RunnableNode,
    code: Code | None,
    text: Text | None,
    options: RunOptions | None,
) -> type[Runner] | None:
    """Gets the runner for the given Run configuration."""
    if kind == RunKind.CODE:
        from bench.runtime.code import CodeFunctionRunner, CodeScriptRunner

        code = code or node.code or Code.empty()
        if (isinstance(node, Block) and node.has_function_fields) or isinstance(node, Step):
            return CodeFunctionRunner
        else:
            return CodeScriptRunner
    elif kind == RunKind.TEXT:
        from bench.runtime.text import AnthropicModelRunner, OpenaiModelRunner, TextRunner

        if options is not None and options.model_provider:
            provider = options.model_provider
        elif options is not None and options.model_type:
            provider = options.model_type.provider
        else:
            provider = None

        if provider is not None:
            if provider == ModelProvider.OPENAI:
                return OpenaiModelRunner
            elif provider == ModelProvider.ANTHROPIC:
                return AnthropicModelRunner
            else:
                return None
        else:
            return TextRunner
    elif kind == RunKind.STEP:
        from bench.runtime.flow import (
            BlockStepRunner,
            CodeStepRunner,
            CompleteStepRunner,
            StartStepRunner,
            TextStepRunner,
        )

        if not isinstance(node, Step):
            raise ValueError(f"unexpected node type for run kind {kind}: {node!r}")
        if node.type == StepType.START:
            return StartStepRunner
        elif node.type == StepType.COMPLETE:
            return CompleteStepRunner
        elif node.type == StepType.BLOCK:
            return BlockStepRunner
        elif node.type == StepType.CODE:
            return CodeStepRunner
        elif node.type == StepType.TEXT:
            return TextStepRunner
        else:
            return None
    elif kind == RunKind.FLOW:
        from bench.runtime.flow import FlowRunner

        return FlowRunner
    else:
        assert_never(kind)
