from typing import TYPE_CHECKING

import black
import structlog

from bench.language.const import EnumType, StructType, enum_
from bench.language.node import Struct, struct_
from bench.language.property import p_regular
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


@enum_(EnumType.CODE_KIND)
class CodeKind(IdEnum):
    """
    The implicit 'kind' of some Code.
    We don't set this explicitly in Code because it depends on where the Code is used.
    """

    SNIPPET = 1  # can import
    SCRIPT = 2  # can import and export
    FUNCTION = 3  # can import, take inputs and produce outputs


@struct_(StructType.CODE_LINE)
class CodeLine(Struct):
    """A line of code."""

    content: str | None = p_regular(32, require=False)


@struct_(StructType.CODE)
class Code(Struct):
    """Code composed of multiple lines."""

    # language: ...
    lines: list[CodeLine] = p_regular(30, require=True, array=True, struct=StructType.CODE_LINE)

    def __content_str__(self) -> str:
        preview_str = "\\n".join(line.content or "" for line in self.lines[:3])
        if len(preview_str) > 100:
            preview_str = preview_str[:100] + "..."
        return f"'{preview_str}', {len(self.lines)} lines"

    def to_string(self) -> str:
        return code_to_string(self)

    @staticmethod
    def from_string(s: str) -> "Code":
        return string_to_code(s)

    @staticmethod
    def empty() -> "Code":
        return Code(lines=[])


def code_to_string(code: Code) -> str:
    return "\n".join(line.content or "" for line in code.lines)


def string_to_code(s: str) -> Code:
    return Code(lines=[CodeLine(content=line or None) for line in s.split("\n")])


def format_code(code: str, suppress_error: bool = False) -> str:
    """Formats the code string with our standard black settings."""
    try:
        return black.format_str(code, mode=black.FileMode(line_length=100))
    except Exception as e:
        if suppress_error:
            return code
        else:
            raise ValueError(code) from e
