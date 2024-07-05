import base64
import dataclasses
import datetime
from dataclasses import dataclass
from datetime import timedelta
from typing import Any
from uuid import UUID

from bench import language
from bench.language.block import Block
from bench.language.code import Code, CodeKind
from bench.language.const import BlockType, RunKind
from bench.language.node import Node
from bench.language.run import Run, RunAttempt, RunOptions
from bench.language.session import Session
from bench.language.setup import BENCH_CLASS_BY_NAME
from bench.language.text import Text
from bench.language.value import ValueObject
from bench.runtime.compiler import CompiledCode, compiled_code
from bench.utils.oracle import Oracle

DEFAULT_CODE_RUN_OPTIONS = RunOptions(max_attempts=1)
DEFAULT_TEXT_RUN_OPTIONS = RunOptions(max_attempts=3, retry_interval=3, backoff=2)
DEFAULT_FLOW_RUN_OPTIONS = RunOptions(max_attempts=1)

# all bench types
CODE_GLOBALS: dict[str, Any] = {**vars(language), **BENCH_CLASS_BY_NAME}
# and some general stuff
for t in (datetime, timedelta, UUID, base64):
    CODE_GLOBALS[t.__name__] = t


@dataclass
class RunContext:
    """The context for any Run (tracked or untacked)."""

    scope: Node
    options: RunOptions
    attempts: list[RunAttempt] = dataclasses.field(default_factory=list)
    compiled: CompiledCode | None = None
    variables: ValueObject | None = None
    inputs: ValueObject | None = None
    run: Run | None = None


class RuntimeState:
    """The overall state of a runtime."""

    def __init__(self, *, session: Session):
        self.session = session


class FlowState:
    """The state of a specific Flow run."""

    def __init__(self, *, flow: Block, run: Run, context: RunContext):
        self.flow = flow
        self.run = run
        self.context = context


class RuntimeRunner:
    """
    A runner processes one top-level Run (or mini run for snippets) at a time.
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
    # Low level stuff
    #

    ...

    #
    # Direct running
    #

    async def run_code_snippet(self, code: Code, context: RunContext):
        """
        Runs Code as a snippet in some context, respecting the run options.
        Snippet runs are lightweight and do not generate tracked Runs.
        """
        raise NotImplementedError

    async def run_code_script(self, code: Code, context: RunContext):
        """Runs Code as a script to store its definitions for reuse, respecting the run options."""
        raise NotImplementedError

    async def run_code_function(self, code: Code, context: RunContext) -> ValueObject:
        """Runs Code with some arguments to produce outputs, respecting the run options."""
        if context.compiled is None:
            context.compiled = compiled_code(
                code.id, code.to_string(), CodeKind.FUNCTION, self.glbls
            )
        raise NotImplementedError("nocheckin: run_code_function")

    async def run_text_function(self, text: Text, context: RunContext) -> ValueObject:
        """Runs Text with inputs, respecting the run options."""
        raise NotImplementedError("nocheckin: run_text_function")

    async def run_flow(self, flow: Block, context: RunContext) -> ValueObject:
        """Runs a Flow (Block) with some inputs to produce outputs, respecting the run options."""
        raise NotImplementedError

    #
    # High level Run control
    #

    async def start_run(self, run: Run):
        """Starts a new Run in this Runner."""
        if run.kind == RunKind.BLOCK:
            block = run.block
            assert block is not None, f"run {run!r} has no block"
            if block.type == BlockType.CODE:
                context = RunContext(
                    scope=block, options=run.options or DEFAULT_CODE_RUN_OPTIONS, inputs=run.inputs
                )
                code = block.code or Code.empty()
                await self.run_code_script(code, context)
            else:
                raise RuntimeError(f"block is not runnable: {block!r}")
        else:
            raise RuntimeError(f"unexpected run: {run!r}")

    async def pause_run(self, run: Run):
        """Pause a Run currently executing in this Runner. Currently only for Flows."""
        raise NotImplementedError

    async def resume_run(self, run: Run):
        """Resume a paused Run in this Runner (maybe from elsewhere). Currently only for Flows."""
        raise NotImplementedError

    async def abort_run(self, run: Run):
        """Abort a Run currently executing in this Runner."""
        raise NotImplementedError
