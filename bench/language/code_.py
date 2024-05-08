import base64
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any
from uuid import UUID

import black
import structlog

from bench.language.const import StructType
from bench.language.node import Struct, struct
from bench.language.property import p_regular

if TYPE_CHECKING:
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


_CODE_GLOBALS: dict[str, Any] | None = None


def get_code_globals():
    global _CODE_GLOBALS
    if _CODE_GLOBALS is None:
        import bench.language
        from bench.language.setup import BENCH_CLASS_BY_NAME

        # all bench types
        _CODE_GLOBALS = {**vars(bench.language), **BENCH_CLASS_BY_NAME}
        # and some general stuff
        for t in (datetime, timedelta, UUID, base64):
            _CODE_GLOBALS[t.__name__] = t
    return _CODE_GLOBALS


def run_code_script(code: str, globals: dict[str, Any] | None = None) -> dict[str, Any]:
    """Runs the code string and extracts its definitions."""
    if globals is None:
        globals = get_code_globals()
    globals_local = {**globals}
    exec(code, globals_local)
    new_globals = {k: v for k, v in globals_local.items() if k not in globals}
    return new_globals


def run_code_eval(code: str, globals: dict[str, Any] | None = None) -> Any:
    """Runs the code string and extracts its definitions."""
    if globals is None:
        globals = get_code_globals()
    return eval(code, globals)
