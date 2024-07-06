from typing import Any, Awaitable

import structlog
from opentelemetry import trace

from bench.language.code import Code
from bench.language.const import BenchError, BlockType, FieldZone, RunErrorKind, RunKind, RunStatus
from bench.language.run import Run, RunAttempt, RunError
from bench.language.session import Session
from bench.language.text import Text
from bench.language.value import ValueObject
from bench.runtime.core import (
    DEFAULT_CODE_RUN_OPTIONS,
    NotRunnableError,
    RunHandle,
    RunnableState,
    RuntimeState,
)
from bench.utils.oracle import Oracle

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class RuntimeRunner:
    """
    A runner processes one top-level Run (or mini run for snippets) at a time (for now).
    Nested runs may run in parallel if they are ready and read-only.
    Caches analyzed/compiled code, maintains outputs, computed expressions, etc..
    """

    def __init__(
        self,
        *,
        state: RuntimeState,
        session: Session,
        oracle: Oracle,
    ):
        self.state = state
        self.session = session
        self.oracle = oracle
        self.glbls = state.glbls

    #
    # Internals
    #

    async def _wrap_tracked_run[R](self, coro: Awaitable[R], handle: RunHandle) -> R:
        """
        Runs a coroutine as a tracked Run with some options for retrying attempts.
        NOTE :Architecture: Run started/terminated/... epochs are relative to session
         (meaning if we make local edits during a Run, the terminated_epoch is still the same)
        """
        run = handle.run
        assert run is not None, f"missing run for context {handle!r}"

        # update context
        run.client = self.session.client
        run.machine = self.session.machine
        run.server = self.session.server
        run.user = self.session.user

        # start if not yet started
        if not run.started_at:
            run.started_epoch = self.session.epoch
            run.started_at = self.oracle.utc()
        run.status = run.current_status = RunStatus.RUNNING

        # actually attempt Run
        retry = handle.options.to_retry().new(self.oracle, attempt=len(handle.attempts))
        try:
            while retry.should_retry:
                retry.on_attempt()
                attempt = RunAttempt(
                    status=RunStatus.RUNNING,
                    started_at=self.oracle.utc(),
                    started_epoch=self.session.epoch,
                )
                handle.attempts.append(attempt)
                try:
                    result = await coro
                    attempt.status = RunStatus.COMPLETED
                    return result
                except Exception as e:
                    attempt.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                    attempt.status = RunStatus.FAILED
                    if not retry.on_error(e):
                        raise
                finally:
                    attempt.terminated_at = self.oracle.utc()
                    attempt.terminated_epoch = self.session.epoch
                    assert attempt.started_at, f"missing started_at for attempt {attempt!r}"
                    attempt.duration = (attempt.terminated_at - attempt.started_at).total_seconds()
            else:
                raise retry.to_error()
        finally:
            # run outcome = status of last attempt
            last_attempt = handle.attempt
            assert last_attempt is not None, f"missing last attempt for run {handle!r}"
            run.attempts = handle.attempts
            run.error = last_attempt.error
            run.status = run.current_status = last_attempt.status
            run.terminated_at = last_attempt.terminated_at
            run.terminated_epoch = last_attempt.terminated_epoch

    @tracer.start_as_current_span("runner.run_code_snippet")
    async def _do_run_code_snippet(self, code: Code, handle: RunHandle) -> None:
        raise NotImplementedError

    @tracer.start_as_current_span("runner.run_code_script")
    async def _do_run_code_script(self, code: Code, handle: RunHandle) -> None:
        """
        Attempts to run a code script once in a prepared context.
        """
        compiled = handle.compiled
        assert compiled, f"missing compiled code for {handle!r}"
        assert compiled.body_co is not None, f"missing compiled body for {handle!r}"
        tmp_glbls = {**self.glbls, "self": handle.scope}  # nocheckin: assemble context properly
        if compiled.is_coroutine:
            await eval(compiled.body_co, tmp_glbls)
        else:
            exec(compiled.body_co, tmp_glbls)

    @tracer.start_as_current_span("runner.run_code_function")
    async def _do_run_code_function(self, code: Code, handle: RunHandle) -> ValueObject:
        raise NotImplementedError("nocheckin: _do_run_code_function")

    #
    # Run stuff directly
    # NOTE :Robustness :Architecture: should we separate transactions for session and user edits?
    # NOTE :Incomplete: respect RunOptions.max_concurrency
    #

    async def run_code_snippet(self, code: Code, handle: RunHandle) -> Any:
        """
        Runs Code as a snippet in some context, respecting the run options.
        Snippet runs are lightweight and do not generate tracked Runs by themself.
        Returns the last expression value.
        """
        raise NotImplementedError

    async def run_code_script(self, code: Code, handle: RunHandle) -> None:
        """Runs Code as a script to store its definitions for reuse, respecting the run options."""
        raise NotImplementedError("nocheckin: run_code_script")

    async def run_code_function(self, code: Code, handle: RunHandle) -> ValueObject:
        """Runs Code with some arguments to produce outputs, respecting the run options."""
        raise NotImplementedError("nocheckin: run_code_function")

    async def run_text_function(self, text: Text, handle: RunHandle) -> ValueObject:
        """Runs Text with inputs, respecting the run options."""
        raise NotImplementedError("nocheckin: run_text_function")

    #
    # High level Run helpers
    #

    async def start_run(self, run: Run):
        """Starts a new top-level Run in this Runner. Returns on termination (?)."""
        # NOTE: Robustness: don't commit the entire session on failure?
        try:
            if run.kind == RunKind.BLOCK:
                block = run.block
                if block is None:
                    raise NotRunnableError(f"run {run!r} has no block")
                if block.type == BlockType.CODE:
                    code = block.code or Code.empty()
                    runnable = RunnableState(id=block.id, scope=block, code=code, text=None)
                    handle = RunHandle(
                        id=run.id,
                        runnable=runnable,
                        options=run.options or DEFAULT_CODE_RUN_OPTIONS,
                        parent=None,
                        status=run.status,
                        inputs=run.inputs,
                        attempts=list(run.attempts),
                    )
                    if any(
                        f.zone == FieldZone.INPUT or f.zone == FieldZone.OUTPUT
                        for f in block.fields
                    ):
                        await self.run_code_function(code, handle)
                    else:
                        await self.run_code_script(code, handle)
                else:
                    raise NotRunnableError(f"block is not runnable: {block!r}")
            else:
                raise NotRunnableError(f"unexpected run: {run!r}")
        except BenchError as e:
            # re-raised inner error
            async with self.session.unsuspended():
                if run.current_status != RunStatus.FAILED:  # failed to attempt
                    run.fail(RunError.from_exception(RunErrorKind.RUNTIME, e))
                await self.session.commit()
            logger.error("runner.start_run.error", run=run, exc_info=e)
        except Exception as e:
            # some internal error
            async with self.session.unsuspended():
                if run.current_status != RunStatus.FAILED:  # failed to attempt
                    run.fail(RunError.from_exception(RunErrorKind.INTERNAL, e))
                await self.session.commit()
            logger.error("runner.start_run.internal_error", run=run, exc_info=e)

    async def pause_run(self, run: Run):
        """Pause a Run currently executing in this Runner."""
        raise NotImplementedError

    async def resume_run(self, run: Run):
        """Resume a paused Run in this Runner (maybe from elsewhere)."""
        raise NotImplementedError

    async def abort_run(self, run: Run):
        """Abort a Run currently executing in this Runner."""
        raise NotImplementedError
