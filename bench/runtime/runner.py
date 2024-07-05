import base64
import dataclasses
import datetime
from dataclasses import dataclass
from datetime import timedelta
from typing import Any, Awaitable
from uuid import UUID

import structlog
from opentelemetry import trace

from bench import language
from bench.language.block import Block
from bench.language.code import Code, CodeKind
from bench.language.const import BlockType, FieldZone, RunKind, RunStatus
from bench.language.node import Node
from bench.language.run import Run, RunAttempt, RunError, RunOptions
from bench.language.session import Session, unsuspend_session
from bench.language.setup import BENCH_CLASS_BY_NAME
from bench.language.text import Text
from bench.language.value import ValueObject
from bench.runtime.compiler import CodeCompilation, compiled_code
from bench.runtime.core import NotRunnableError
from bench.utils.oracle import Oracle

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

DEFAULT_CODE_RUN_OPTIONS = RunOptions(max_attempts=1)
DEFAULT_TEXT_RUN_OPTIONS = RunOptions(max_attempts=3, retry_interval=3, backoff=2)
DEFAULT_FLOW_RUN_OPTIONS = RunOptions(max_attempts=1)

# all bench types
CODE_GLOBALS: dict[str, Any] = {**vars(language), **BENCH_CLASS_BY_NAME}
# and some general stuff
for t in (datetime, timedelta, UUID, base64):
    CODE_GLOBALS[t.__name__] = t


@dataclass(slots=True)
class RunContext:
    """The context for any Run (tracked or untacked)."""

    scope: Node
    options: RunOptions
    attempts: list[RunAttempt] = dataclasses.field(default_factory=list)
    compiled: CodeCompilation | None = None
    variables: ValueObject | None = None
    inputs: ValueObject | None = None
    run: Run | None = None

    def __str__(self):
        context_parts: list[str] = [f"scope={self.scope!r}", f"options={self.options!r}"]
        if self.attempts:
            context_parts.append(f"attempts={self.attempts}")
        if self.compiled:
            context_parts.append(f"compiled={self.compiled}")
        if self.run:
            context_parts.append(f"run={self.run}")
        return ", ".join(context_parts)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}"

    @property
    def attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None


class RuntimeState:
    """The overall state of a runtime."""

    def __init__(self, *, session: Session):
        self.session = session


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
        glbls: dict[str, Any] = CODE_GLOBALS,
    ):
        self.state = state
        self.session = session
        self.oracle = oracle
        self.glbls = glbls

    #
    # Internals
    #

    async def _prepare_code_context(self, context: RunContext):
        """Prepares references and such to assemble the globals for a Run (recursively)."""
        pass  # nocheckin

    async def _wrap_tracked_run[R](self, coro: Awaitable[R], context: RunContext) -> R:
        """Runs a coroutine as a tracked Run with some options."""
        run = context.run
        assert run is not None, f"missing run for context {context!r}"

        # update context
        run.client = self.session.client
        run.machine = self.session.machine
        run.server = self.session.server
        run.user = self.session.user

        # start if not yet started
        if not run.started_at:
            run.started_at = self.oracle.utc()
            run.started_epoch = self.session.epoch
            run.status = RunStatus.RUNNING

        # actually attempt Run
        retry = context.options.to_retry().new(self.oracle, attempt=len(context.attempts))
        try:
            while retry.should_retry:
                retry.on_attempt()
                attempt = RunAttempt(
                    status=RunStatus.RUNNING,
                    started_at=self.oracle.utc(),
                    started_epoch=self.session.epoch,
                )
                try:
                    result = await coro
                    attempt.status = RunStatus.COMPLETED
                    return result
                except Exception as e:
                    attempt.error = RunError.from_exception(e)
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
            # run status = status of last attempt
            last_attempt = context.attempt
            assert last_attempt is not None, f"missing last attempt for run {context!r}"
            run.attempts = context.attempts
            run.error = last_attempt.error
            run.status = last_attempt.status
            run.terminated_at = last_attempt.terminated_at
            run.terminated_epoch = last_attempt.terminated_epoch

    async def _do_run_code_script(self, code: Code, context: RunContext) -> None:
        """Attempts to run a code script once with a prepared context."""
        raise NotImplementedError("nocheckin: _do_run_code_script")

    async def _do_run_code_function(self, code: Code, context: RunContext) -> ValueObject:
        raise NotImplementedError("nocheckin: _do_run_code_function")

    ...

    #
    # Run stuff directly
    # NOTE :Robustness :Architecture: should we separate transactions for session and user edits?
    #

    async def run_code_snippet(self, code: Code, context: RunContext) -> Any:
        """
        Runs Code as a snippet in some context, respecting the run options.
        Snippet runs are lightweight and do not generate tracked Runs by themself.
        Returns the last expression value.
        """
        if context.compiled is None:
            context.compiled = compiled_code(
                code.id, code.to_string(), CodeKind.SNIPPET, self.glbls
            )
        await self._prepare_code_context(context)
        raise NotImplementedError

    async def run_code_script(self, code: Code, context: RunContext) -> None:
        """Runs Code as a script to store its definitions for reuse, respecting the run options."""
        if context.compiled is None:
            context.compiled = compiled_code(code.id, code.to_string(), CodeKind.SCRIPT, self.glbls)
        await self._prepare_code_context(context)
        async with unsuspend_session(self.session):
            await self._wrap_tracked_run(self._do_run_code_function(code, context), context)

    async def run_code_function(self, code: Code, context: RunContext) -> ValueObject:
        """Runs Code with some arguments to produce outputs, respecting the run options."""
        if context.compiled is None:
            context.compiled = compiled_code(
                code.id, code.to_string(), CodeKind.FUNCTION, self.glbls
            )
        await self._prepare_code_context(context)
        async with unsuspend_session(self.session):
            return await self._wrap_tracked_run(self._do_run_code_function(code, context), context)

    async def run_text_function(self, text: Text, context: RunContext) -> ValueObject:
        """Runs Text with inputs, respecting the run options."""
        raise NotImplementedError("nocheckin: run_text_function")

    async def run_flow(self, flow: Block, context: RunContext) -> ValueObject:
        """Runs a Flow (Block) with some inputs to produce outputs, respecting the run options."""
        raise NotImplementedError

    #
    # High level Run helpers
    #

    async def start_run(self, run: Run):
        """Starts a new top-level Run in this Runner. Returns on termination (?)."""
        try:
            if run.kind == RunKind.BLOCK:
                block = run.block
                assert block is not None, f"run {run!r} has no block"
                if block.type == BlockType.CODE:
                    context = RunContext(
                        scope=block,
                        options=run.options or DEFAULT_CODE_RUN_OPTIONS,
                        inputs=run.inputs,
                        run=run,
                    )
                    code = block.code or Code.empty()
                    if any(
                        f.zone == FieldZone.INPUT or f.zone == FieldZone.OUTPUT
                        for f in block.fields
                    ):
                        await self.run_code_function(code, context)
                    else:
                        await self.run_code_script(code, context)
                else:
                    raise NotRunnableError(f"block is not runnable: {block!r}")
            else:
                raise NotRunnableError(f"unexpected run: {run!r}")
        except Exception as e:
            # failed to even attempt
            # NOTE :Robustness: do we really want to turn *every* error into a failed Run?
            async with unsuspend_session(self.session, readonly=False):
                run.fail(RunError.from_exception(e))
                await self.session.commit()  # nocheckin

    async def pause_run(self, run: Run):
        """Pause a Run currently executing in this Runner. Currently only for top-level or nested Flows."""
        raise NotImplementedError

    async def resume_run(self, run: Run):
        """Resume a paused Run in this Runner (maybe from elsewhere). See pause_run."""
        raise NotImplementedError

    async def abort_run(self, run: Run):
        """Abort a Run currently executing in this Runner."""
        raise NotImplementedError
