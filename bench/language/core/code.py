import textwrap
from typing import TYPE_CHECKING

import black
import structlog
from opentelemetry import trace

from bench.utils.func import IdEnum

from .const import EnumType, StructType, enum_
from .node import Struct, struct_
from .property import p_regular

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@enum_(EnumType.CODE_TYPE)
class CodeType(IdEnum):
    """
    The implicit 'type' of some Code.
    We don't set this explicitly in Code because it depends on where the Code is used.
    """

    SNIPPET = 1  # inline expressions and procedures anywhere
    SCRIPT = 2  # define Python-level commons 'scripts'
    FUNCTION = 3  # Python functions


@struct_(StructType.CODE_LINE)
class CodeLine(Struct):
    """A line of code."""

    content: str | None = p_regular(32, require=False)

    def __len__(self) -> int:
        if self.content is None:
            return 0
        else:
            return len(self.content)

    def __contains__(self, other: str) -> bool:
        if self.content is None:
            return False
        else:
            return other in self.content


@struct_(StructType.CODE)
class Code(Struct):
    """Code composed of multiple lines."""

    # TODO :Incomplete: support references in nodes (incl. Paths? also in Text?)

    # language: ...
    lines: list[CodeLine] = p_regular(30, require=True, array=True, struct=StructType.CODE_LINE)

    def __content_str__(self) -> str:
        preview_str = "\\n".join(line.content or "" for line in self.lines[:3])
        if len(preview_str) > 100:
            preview_str = preview_str[:100] + "..."
        return f"'{preview_str}', {len(self.lines)} lines"

    def __len__(self) -> int:
        return sum(len(line) for line in self.lines)

    def __contains__(self, other: str) -> bool:
        return any(other in line for line in self.lines)

    def to_string(self) -> str:
        return "\n".join(line.content or "" for line in self.lines)

    @staticmethod
    def from_string(s: str) -> "Code":
        if not s:
            return Code.empty()
        s = textwrap.dedent(s)
        lines = [CodeLine(content=line or None) for line in s.split("\n")]
        return Code(lines=lines)

    @staticmethod
    def empty() -> "Code":
        return Code(lines=[])


code = Code.from_string


@tracer.start_as_current_span(name="code.format")
def format_code(code: str, suppress_error: bool = False, line_length: int = 100) -> str:
    """
    Formats the code string with our standard black settings.
    TODO :Performance!: replace black with ruff in format_code
     (unfortunately ruff doesn't have a nice API for this yet, so we would need to use a subprocess?)
    """
    try:
        return black.format_str(code, mode=black.FileMode(line_length=line_length))
    except Exception as e:
        if suppress_error:
            return code
        else:
            raise SyntaxError(code) from e
