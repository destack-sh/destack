import typing

import black
import structlog

from bench.language.const import StructType
from bench.language.node import Struct, struct
from bench.language.property import p_regular

if typing.TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


@struct(StructType.CODE_LINE)
class CodeLine(Struct):
    content: str = p_regular(32)


@struct(StructType.CODE)
class Code(Struct):
    # language: ...
    lines: list[CodeLine] = p_regular(30, require=True, array=True, struct=StructType.CODE_LINE)


def code_to_string(code: Code) -> str:
    return "\n".join(line.content for line in code.lines)


def string_to_code(s: str) -> Code:
    return Code(lines=[CodeLine(content=line) for line in s.split("\n")])


def format_code(code: str, suppress_error: bool = False) -> str:
    """Formats the code string with our standard black settings."""
    try:
        return black.format_str(code, mode=black.FileMode(line_length=100))
    except Exception as e:
        if suppress_error:
            return code
        else:
            raise ValueError(code) from e
