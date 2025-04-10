import codeop
import textwrap
from typing import Any

import regex
import structlog
from opentelemetry import trace

from bench.language import Aliasing
from bench.runtime.code.context import STATIC_CODE_GLOBALS
from bench.runtime.core.runner import Runner
from bench.runtime.model.macro import MACROS_BY_NAME

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class StreamingCodeRunner:
    def __init__(self, runner: Runner, aliasing: Aliasing, code: str = ""):
        self.runner = runner
        self.aliasing = aliasing
        self.code = code
        self.pending_code = code
        self.globals: dict[str, Any] = {
            **STATIC_CODE_GLOBALS,
            **{macro.name: macro.bind(runner) for macro in MACROS_BY_NAME.values()},
            **(aliasing._node_by_alias if aliasing else {}),
        }

    @tracer.start_as_current_span("streaming_code_runner.execute")
    def _execute(self, code: str) -> None:
        """Execute code."""
        try:
            exec(code, self.globals)
            logger.trace("streaming_code_runner.execute", code=code, span="current")
        except Exception as e:
            logger.error("streaming_code_runner.error", code=code, span="current", exc_info=e)
            raise

    def _is_valid(self, code: str) -> bool:
        """Check if the code is (syntactically) complete."""
        try:
            return codeop.compile_command(code, symbol="exec") is not None
        except Exception:
            return False

    def add(self, new_code: str) -> None:
        """Adds code and executes it (if complete & valid)."""
        self.code += new_code
        self.pending_code += new_code
        if self.pending_code.endswith("\n") and self._is_valid(self.pending_code):
            self._execute(self.pending_code)
            self.pending_code = ""

    def complete(self) -> None:
        """Finish running the code. Raise if there is trailing unexecuted (=invalid) code."""
        if self.pending_code:
            if self._is_valid(self.pending_code):
                self._execute(self.pending_code)
                self.pending_code = ""
            else:
                raise RuntimeError(f"bad trailing code: {self.pending_code!r}")


def clean_code(code: str) -> str:
    """Standardize code completion."""
    # clean completion
    code = code.strip()
    # strip ``` ... ``` wrapper
    code = regex.sub(r"^```[a-zA-Z]*\n", "", code)
    code = regex.sub(r"\n```$", "", code)
    # replace suspicious unicode characters
    code = code.replace("’", "'")  # noqa: RUF001
    code = code.replace("‘", "'")  # noqa: RUF001
    code = code.replace("“", '"')
    code = code.replace("”", '"')
    # dedent
    code = textwrap.dedent(code)
    return code
