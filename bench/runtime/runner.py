import base64
import datetime
from datetime import timedelta
from typing import Any
from uuid import UUID

from bench import language
from bench.language.block import Block
from bench.language.code import Code
from bench.language.run import RunOptions
from bench.language.session import Session
from bench.language.setup import BENCH_CLASS_BY_NAME
from bench.language.text import Text
from bench.language.value import ValueObject

DEFAULT_CODE_RUN_OPTIONS = RunOptions(max_attempts=1)
DEFAULT_TEXT_RUN_OPTIONS = RunOptions(max_attempts=3, retry_interval=3, backoff=2)

# all bench types
CODE_GLOBALS: dict[str, Any] = {**vars(language), **BENCH_CLASS_BY_NAME}
# and some general stuff
for t in (datetime, timedelta, UUID, base64):
    CODE_GLOBALS[t.__name__] = t


class RuntimeState:
    """The state of a runtime."""

    def __init__(self, *, session: Session):
        self.session = session


class RuntimeRunner:
    """
    Actually run Code, Text, Flows, etc. in a Runtime.
    Caches compiled code, some outputs, etc.
    """

    def __init__(self, *, state: RuntimeState, session: Session):
        self.state = state
        self.session = session

    async def run_code_snippet(self, code: Code, options: RunOptions):
        """Runs Code as a snippet in some context, respecting the run options."""
        raise NotImplementedError

    async def run_code_script(self, code: Code, options: RunOptions):
        """Runs Code as a script to store its definitions for reuse, respecting the run options."""
        raise NotImplementedError

    async def run_code_function(
        self, code: Code, inputs: ValueObject, options: RunOptions
    ) -> ValueObject:
        """Runs Code with some arguments to produce outputs, respecting the run options."""
        raise NotImplementedError("nocheckin: run_code_function")

    async def run_text_function(self, text: Text, options: RunOptions) -> ValueObject:
        """Runs Text with inputs"""
        raise NotImplementedError("nocheckin: run_text_function")

    async def run_flow(self, flow: Block, inputs: ValueObject, options: RunOptions) -> ValueObject:
        """Runs a Flow (Block) with some inputs to produce outputs, respecting the run options."""
        raise NotImplementedError
